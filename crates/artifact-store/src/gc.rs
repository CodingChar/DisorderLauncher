use crate::index::ArtifactIndex;
use crate::{ArtifactError, ArtifactStore};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Gracia por defecto: un objeto recien llegado puede estar a punto de anotarse en una instancia.
pub const DEFAULT_GRACE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoreStats {
    pub objects: usize,
    pub store_bytes: u64,
    /// Bytes que se ahorran por compartir: lo que ocuparian los archivos de todas las instancias
    /// si cada una tuviera su copia, menos lo que ocupan los objetos distintos.
    pub shared_bytes: u64,
    /// Objetos que ninguna instancia usa (y han pasado la gracia): `gc` los libera.
    pub orphan_objects: usize,
    pub orphan_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcReport {
    pub removed: usize,
    pub freed_bytes: u64,
}

/// Objetos usados por las instancias, con cuantas referencias tiene cada uno y el tamano total que sumarian.
struct References {
    hashes: HashSet<String>,
    logical_bytes: u64,
}

/// Lee el indice de cada instancia. Si alguno no se puede interpretar no se sabe que usa esa instancia,
/// y borrar a ciegas podria quitarle un mod: se rechaza toda la operacion.
fn collect(game_dirs: &[PathBuf]) -> Result<References, ArtifactError> {
    let mut hashes = HashSet::new();
    let mut logical_bytes = 0u64;
    for dir in game_dirs {
        if ArtifactIndex::is_unreadable(dir) {
            return Err(ArtifactError::UnreadableIndex);
        }
        for entry in ArtifactIndex::load(dir).entries.values() {
            hashes.insert(entry.sha512.clone());
            logical_bytes += entry.size;
        }
    }
    Ok(References { hashes, logical_bytes })
}

fn is_old(modified: SystemTime, grace: Duration) -> bool {
    SystemTime::now().duration_since(modified).map(|age| age >= grace).unwrap_or(false)
}

pub fn stats(store: &ArtifactStore, game_dirs: &[PathBuf], grace: Duration) -> Result<StoreStats, ArtifactError> {
    let refs = collect(game_dirs)?;
    let objects = store.list()?;
    let mut out = StoreStats { objects: objects.len(), ..Default::default() };
    let mut referenced_physical = 0u64;
    for object in &objects {
        out.store_bytes += object.size;
        if refs.hashes.contains(&object.hash) {
            referenced_physical += object.size;
        } else if is_old(object.modified, grace) {
            out.orphan_objects += 1;
            out.orphan_bytes += object.size;
        }
    }
    out.shared_bytes = refs.logical_bytes.saturating_sub(referenced_physical);
    Ok(out)
}

/// Borra del almacen los objetos que ninguna instancia usa, pasada la gracia, y los temporales abandonados.
pub fn gc(store: &ArtifactStore, game_dirs: &[PathBuf], grace: Duration) -> Result<GcReport, ArtifactError> {
    let refs = collect(game_dirs)?;
    let mut report = GcReport::default();
    for object in store.list()? {
        if refs.hashes.contains(&object.hash) || !is_old(object.modified, grace) {
            continue;
        }
        match fs::remove_file(&object.path) {
            Ok(()) => {
                report.removed += 1;
                report.freed_bytes += object.size;
            }
            Err(e) => tracing::warn!(error = %e, "no se pudo borrar un objeto sin uso del almacen"),
        }
    }
    if let Ok(read) = fs::read_dir(store.tmp_dir()) {
        for entry in read.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() && meta.modified().is_ok_and(|m| is_old(m, grace)) {
                let size = meta.len();
                if fs::remove_file(entry.path()).is_ok() {
                    report.freed_bytes += size;
                }
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adopt_instance, ArtifactStore};

    fn setup() -> (tempfile::TempDir, ArtifactStore, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ArtifactStore::open(tmp.path().join("store")).unwrap();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        for dir in [&a, &b] {
            fs::create_dir_all(dir.join("mods")).unwrap();
        }
        (tmp, store, a, b)
    }

    fn adopt(store: &ArtifactStore, dir: &std::path::Path) {
        adopt_instance(store, dir, &|_, _| {}).unwrap();
    }

    #[test]
    fn unused_objects_are_freed_but_shared_ones_stay() {
        let (_tmp, store, a, b) = setup();
        fs::write(a.join("mods/shared.jar"), vec![1u8; 1000]).unwrap();
        fs::write(b.join("mods/shared.jar"), vec![1u8; 1000]).unwrap();
        fs::write(a.join("mods/only-a.jar"), vec![2u8; 400]).unwrap();
        adopt(&store, &a);
        adopt(&store, &b);
        let dirs = vec![a.clone(), b.clone()];

        let before = stats(&store, &dirs, Duration::ZERO).unwrap();
        assert_eq!((before.objects, before.store_bytes, before.shared_bytes, before.orphan_objects), (2, 1400, 1000, 0));

        // Se quita el mod exclusivo de A: queda sin uso. El compartido sigue en uso por B.
        fs::remove_file(a.join("mods/only-a.jar")).unwrap();
        adopt(&store, &a);
        let orphaned = stats(&store, &dirs, Duration::ZERO).unwrap();
        assert_eq!((orphaned.orphan_objects, orphaned.orphan_bytes), (1, 400));

        let report = gc(&store, &dirs, Duration::ZERO).unwrap();
        assert_eq!((report.removed, report.freed_bytes), (1, 400));
        assert_eq!(store.list().unwrap().len(), 1);
        assert_eq!(fs::read(b.join("mods/shared.jar")).unwrap(), vec![1u8; 1000]);

        // Quitarlo tambien de B libera el ultimo objeto.
        fs::remove_file(a.join("mods/shared.jar")).unwrap();
        fs::remove_file(b.join("mods/shared.jar")).unwrap();
        adopt(&store, &a);
        adopt(&store, &b);
        assert_eq!(gc(&store, &dirs, Duration::ZERO).unwrap().removed, 1);
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn recent_objects_survive_the_grace_period() {
        let (_tmp, store, a, _b) = setup();
        fs::write(a.join("mods/x.jar"), b"x").unwrap();
        adopt(&store, &a);
        fs::remove_file(a.join("mods/x.jar")).unwrap();
        adopt(&store, &a);
        let report = gc(&store, &[a], DEFAULT_GRACE).unwrap();
        assert_eq!(report.removed, 0, "acaba de llegar: puede estar por anotarse");
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn an_unreadable_index_blocks_the_cleanup() {
        let (_tmp, store, a, b) = setup();
        fs::write(a.join("mods/x.jar"), b"x").unwrap();
        adopt(&store, &a);
        fs::write(b.join(crate::INDEX_FILE), b"{ roto").unwrap_or_else(|_| {
            fs::create_dir_all(b.join(".disorder")).unwrap();
            fs::write(b.join(crate::INDEX_FILE), b"{ roto").unwrap();
        });
        assert!(matches!(gc(&store, &[a.clone(), b.clone()], Duration::ZERO), Err(ArtifactError::UnreadableIndex)));
        assert!(matches!(stats(&store, &[a, b], Duration::ZERO), Err(ArtifactError::UnreadableIndex)));
        assert_eq!(store.list().unwrap().len(), 1, "no se borro nada");
    }

    #[test]
    fn stale_temp_files_are_cleaned() {
        let (_tmp, store, a, _b) = setup();
        fs::write(store.tmp_dir().join("obj-abandonado.part"), vec![0u8; 50]).unwrap();
        let report = gc(&store, &[a], Duration::ZERO).unwrap();
        assert_eq!(report.freed_bytes, 50);
        assert!(!store.tmp_dir().join("obj-abandonado.part").exists());
    }
}
