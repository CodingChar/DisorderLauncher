use crate::index::{ArtifactEntry, ArtifactIndex, ArtifactSource};
use crate::store::{ArtifactStore, Method};
use crate::{hash_file, ArtifactError};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

/// Carpetas con archivos inmutables que se comparten. `config/`, `saves/`, etc. son estado de la instancia.
pub const STORE_DIRS: [&str; 3] = ["mods", "resourcepacks", "shaderpacks"];
const MAX_THREADS: usize = 4;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdoptReport {
    /// Archivos nuevos o cambiados que se incorporaron.
    pub adopted: usize,
    /// Archivos que no cambiaron y no se volvieron a leer.
    pub unchanged: usize,
    /// Entradas del indice cuyo archivo ya no existe.
    pub removed: usize,
    /// Bytes que dejan de estar duplicados porque ahora comparten contenido con el almacen.
    pub saved_bytes: u64,
}

struct Candidate {
    rel: String,
    path: PathBuf,
    size: u64,
    mtime_ns: u64,
}

fn mtime_ns(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn is_shareable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.starts_with('.') && [".jar", ".jar.disabled", ".zip", ".zip.disabled"].iter().any(|ext| lower.ends_with(ext))
}

fn scan(game_dir: &Path) -> Vec<Candidate> {
    let mut out = Vec::new();
    for dir in STORE_DIRS {
        let Ok(read) = fs::read_dir(game_dir.join(dir)) else { continue };
        for entry in read.flatten() {
            let Ok(file_type) = entry.file_type() else { continue };
            let name = entry.file_name().to_string_lossy().into_owned();
            if !file_type.is_file() || !is_shareable(&name) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            out.push(Candidate { rel: format!("{dir}/{name}"), path: entry.path(), size: meta.len(), mtime_ns: mtime_ns(&meta) });
        }
    }
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

/// Hashea en paralelo (acotado) los archivos pendientes. Cada hilo lee un archivo a la vez, en streaming.
fn hash_all(pending: &[&Candidate], progress: &(dyn Fn(usize, usize) + Sync)) -> Vec<Option<(String, u64)>> {
    let results: Mutex<Vec<Option<(String, u64)>>> = Mutex::new(vec![None; pending.len()]);
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, MAX_THREADS)
        .min(pending.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(candidate) = pending.get(i) else { break };
                match hash_file(&candidate.path) {
                    Ok(hashed) => results.lock().unwrap()[i] = Some(hashed),
                    Err(e) => tracing::warn!(file = %candidate.rel, error = %e, "no se pudo leer el archivo para adoptarlo"),
                }
                progress(done.fetch_add(1, Ordering::Relaxed) + 1, pending.len());
            });
        }
    });
    results.into_inner().unwrap()
}

/// Incorpora al almacen los archivos nuevos o cambiados de la instancia y deja su indice al dia.
/// Es sincrona y pesada en disco: llamarla desde `spawn_blocking`. La instancia no debe estar en ejecucion.
pub fn adopt_instance(
    store: &ArtifactStore,
    game_dir: &Path,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<AdoptReport, ArtifactError> {
    let mut index = ArtifactIndex::load(game_dir);
    let candidates = scan(game_dir);
    let mut report = AdoptReport::default();

    // Un archivo renombrado (p. ej. desactivado) conserva el origen de su contenido.
    let known_sources: std::collections::HashMap<String, ArtifactSource> =
        index.entries.values().map(|e| (e.sha512.clone(), e.source.clone())).collect();
    let seen: BTreeSet<&str> = candidates.iter().map(|c| c.rel.as_str()).collect();
    let before = index.entries.len();
    index.entries.retain(|rel, _| seen.contains(rel.as_str()));
    report.removed = before - index.entries.len();

    let pending: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| !matches!(index.entries.get(&c.rel), Some(e) if e.size == c.size && e.mtime_ns == c.mtime_ns))
        .collect();
    report.unchanged = candidates.len() - pending.len();

    let hashes = hash_all(&pending, progress);
    for (candidate, hashed) in pending.iter().zip(hashes) {
        let Some((hash, size)) = hashed else { continue };
        let ingested = match store.ingest_known(&candidate.path, &hash, size) {
            Ok(method) => method,
            Err(e) => {
                tracing::warn!(file = %candidate.rel, error = %e, "no se pudo incorporar al almacen");
                continue;
            }
        };
        // El contenido ya estaba en el almacen: este archivo era una copia; ahora comparte los bytes.
        if ingested.is_none() {
            match store.materialize(&hash, &candidate.path) {
                Ok(Method::Hardlink) => report.saved_bytes += size,
                Ok(Method::Copy) => {}
                Err(e) => tracing::warn!(file = %candidate.rel, error = %e, "no se pudo enlazar con el almacen"),
            }
        }
        let mtime = fs::metadata(&candidate.path).map(|m| mtime_ns(&m)).unwrap_or(candidate.mtime_ns);
        let source = match index.entries.get(&candidate.rel) {
            Some(prev) if prev.sha512 == hash => prev.source.clone(),
            _ => known_sources.get(&hash).cloned().unwrap_or(ArtifactSource::Local),
        };
        index.entries.insert(candidate.rel.clone(), ArtifactEntry { sha512: hash, size, mtime_ns: mtime, source });
        report.adopted += 1;
    }

    if report.adopted > 0 || report.removed > 0 {
        index.save(game_dir)?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash_reader;

    fn setup() -> (tempfile::TempDir, ArtifactStore, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ArtifactStore::open(tmp.path().join("store")).unwrap();
        let game = tmp.path().join("game");
        fs::create_dir_all(game.join("mods")).unwrap();
        (tmp, store, game)
    }

    fn noop(_: usize, _: usize) {}

    #[test]
    fn sha512_matches_known_vector() {
        let (hash, size) = hash_reader(&b"abc"[..]).unwrap();
        assert_eq!(size, 3);
        assert!(hash.starts_with("ddaf35a193617aba"));
        assert!(crate::is_valid_hash(&hash));
    }

    #[test]
    fn rejects_malformed_hashes() {
        let (_tmp, store, _) = setup();
        assert!(store.path_of("../../etc/passwd").is_err());
        assert!(store.path_of(&"A".repeat(128)).is_err());
        assert!(store.path_of(&"a".repeat(127)).is_err());
    }

    #[test]
    fn ingest_is_idempotent() {
        let (_tmp, store, game) = setup();
        let jar = game.join("mods/a.jar");
        fs::write(&jar, b"contenido").unwrap();
        let first = store.ingest_file(&jar).unwrap();
        let second = store.ingest_file(&jar).unwrap();
        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.hash, second.hash);
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn materialize_shares_content_and_survives_link_removal() {
        let (_tmp, store, game) = setup();
        let jar = game.join("mods/a.jar");
        fs::write(&jar, b"mod sodium").unwrap();
        let ingested = store.ingest_file(&jar).unwrap();
        let dest = game.join("mods/copy.jar");
        let method = store.materialize(&ingested.hash, &dest).unwrap();
        assert_eq!(method, Method::Hardlink);
        assert_eq!(fs::read(&dest).unwrap(), b"mod sodium");
        fs::remove_file(&dest).unwrap();
        fs::remove_file(&jar).unwrap();
        assert!(store.verify(&ingested.hash).unwrap(), "el objeto sigue intacto");
    }

    #[test]
    fn adoption_dedupes_copies_across_instances() {
        let (tmp, store, game_a) = setup();
        let game_b = tmp.path().join("game_b");
        fs::create_dir_all(game_b.join("mods")).unwrap();
        fs::write(game_a.join("mods/sodium.jar"), vec![7u8; 4096]).unwrap();
        fs::write(game_b.join("mods/sodium-renamed.jar"), vec![7u8; 4096]).unwrap();
        let a = adopt_instance(&store, &game_a, &noop).unwrap();
        let b = adopt_instance(&store, &game_b, &noop).unwrap();
        assert_eq!((a.adopted, a.saved_bytes), (1, 0));
        assert_eq!((b.adopted, b.saved_bytes), (1, 4096));
        assert_eq!(store.list().unwrap().len(), 1, "un solo objeto fisico");
    }

    #[test]
    fn second_pass_does_not_rehash_and_changes_are_detected() {
        let (_tmp, store, game) = setup();
        let jar = game.join("mods/a.jar");
        fs::write(&jar, b"v1").unwrap();
        assert_eq!(adopt_instance(&store, &game, &noop).unwrap().adopted, 1);
        let again = adopt_instance(&store, &game, &noop).unwrap();
        assert_eq!((again.adopted, again.unchanged), (0, 1));

        // Se reemplaza como hace el launcher o el usuario al copiar un archivo nuevo.
        fs::remove_file(&jar).unwrap();
        fs::write(&jar, b"version dos").unwrap();
        let changed = adopt_instance(&store, &game, &noop).unwrap();
        assert_eq!(changed.adopted, 1);
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn removed_files_leave_the_index_and_unrelated_files_are_ignored() {
        let (_tmp, store, game) = setup();
        fs::write(game.join("mods/a.jar"), b"a").unwrap();
        fs::write(game.join("mods/notes.txt"), b"no es un mod").unwrap();
        fs::write(game.join("mods/.hidden.jar.part"), b"x").unwrap();
        adopt_instance(&store, &game, &noop).unwrap();
        assert_eq!(ArtifactIndex::load(&game).entries.len(), 1);
        fs::remove_file(game.join("mods/a.jar")).unwrap();
        let report = adopt_instance(&store, &game, &noop).unwrap();
        assert_eq!(report.removed, 1);
        assert!(ArtifactIndex::load(&game).entries.is_empty());
    }

    #[test]
    fn renamed_file_keeps_its_known_source() {
        let (_tmp, store, game) = setup();
        fs::write(game.join("mods/a.jar"), b"contenido").unwrap();
        adopt_instance(&store, &game, &noop).unwrap();
        let hash = ArtifactIndex::load(&game).entries["mods/a.jar"].sha512.clone();
        let source = ArtifactSource::Modrinth { project_id: "P".into(), version_id: "V".into() };
        ArtifactIndex::record_file(&game, "mods/a.jar", &hash, source.clone()).unwrap();
        fs::rename(game.join("mods/a.jar"), game.join("mods/a.jar.disabled")).unwrap();
        adopt_instance(&store, &game, &noop).unwrap();
        assert_eq!(ArtifactIndex::load(&game).entries["mods/a.jar.disabled"].source, source);
    }

    #[test]
    fn verify_detects_in_place_tampering() {
        let (_tmp, store, game) = setup();
        let jar = game.join("mods/a.jar");
        fs::write(&jar, b"original").unwrap();
        let ingested = store.ingest_file(&jar).unwrap();
        fs::write(store.path_of(&ingested.hash).unwrap(), b"alterado!").unwrap();
        assert!(!store.verify(&ingested.hash).unwrap());
    }

    #[test]
    fn corrupt_index_is_treated_as_empty() {
        let (_tmp, store, game) = setup();
        fs::create_dir_all(game.join(".disorder")).unwrap();
        fs::write(game.join(crate::INDEX_FILE), b"{ no json").unwrap();
        assert!(ArtifactIndex::is_unreadable(&game));
        fs::write(game.join("mods/a.jar"), b"a").unwrap();
        assert_eq!(adopt_instance(&store, &game, &noop).unwrap().adopted, 1);
        assert!(!ArtifactIndex::is_unreadable(&game));
    }
}
