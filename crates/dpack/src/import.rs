use crate::manifest::{Manifest, ManifestEntry};
use crate::{DpackError, MANIFEST_NAME};
use artifact_store::Sha512Hasher;
use sevenz_rust2::{ArchiveReader, Password};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const COPY_BUFFER: usize = 256 * 1024;
/// Bit de "enlace simbolico" en los atributos de Windows y en el modo Unix que 7z guarda en los 16 bits altos.
const ATTR_REPARSE_POINT: u32 = 0x400;
const ATTR_UNIX_EXTENSION: u32 = 0x8000;
const UNIX_TYPE_MASK: u32 = 0o170000;
const UNIX_SYMLINK: u32 = 0o120000;

/// Topes contra paquetes hostiles (bombas de descompresion, millones de archivos).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_entries: usize,
    pub max_total_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_entries: 50_000, max_total_bytes: 8 * 1024 * 1024 * 1024 }
    }
}

/// Se llama con cada entrada verificada; `None` si no hizo falta escribirla.
type Done<'a> = dyn FnMut(&ManifestEntry, Option<&Path>) -> Result<(), DpackError> + 'a;

/// Que hacer con una entrada del paquete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractPlan {
    /// El contenido ya esta disponible (p. ej. en el almacen): no hace falta escribirlo.
    Skip,
    WriteTo(PathBuf),
}

fn is_link(attributes: u32) -> bool {
    attributes & ATTR_REPARSE_POINT != 0
        || (attributes & ATTR_UNIX_EXTENSION != 0 && (attributes >> 16) & UNIX_TYPE_MASK == UNIX_SYMLINK)
}

fn open(archive: &Path) -> Result<ArchiveReader<File>, DpackError> {
    let file = File::open(archive)?;
    ArchiveReader::new(file, Password::empty()).map_err(|_| DpackError::NotADpack)
}

/// Comprueba las cabeceras del `.7z` antes de descomprimir nada: cuantas entradas y cuanto
/// declaran ocupar. El descompresor limita cada bloque a lo declarado, asi que el tope es real.
fn check_headers(reader: &ArchiveReader<File>, limits: &Limits) -> Result<(), DpackError> {
    let files = &reader.archive().files;
    if files.len() > limits.max_entries + 1 {
        return Err(DpackError::TooManyEntries);
    }
    let mut total = 0u64;
    for file in files {
        total = total.checked_add(file.size()).ok_or(DpackError::TooLarge)?;
    }
    if total > limits.max_total_bytes.saturating_add(MAX_MANIFEST_BYTES) {
        return Err(DpackError::TooLarge);
    }
    Ok(())
}

/// Lee y valida solo el manifiesto. Un archivo que no es un paquete (o no tiene manifiesto) da `NotADpack`.
pub fn read_manifest(archive: &Path, limits: &Limits) -> Result<Manifest, DpackError> {
    let mut reader = open(archive)?;
    check_headers(&reader, limits)?;
    let declared = reader
        .archive()
        .files
        .iter()
        .find(|f| f.name() == MANIFEST_NAME)
        .map(|f| f.size())
        .ok_or(DpackError::NotADpack)?;
    if declared > MAX_MANIFEST_BYTES {
        return Err(DpackError::TooLarge);
    }
    let bytes = reader.read_file(MANIFEST_NAME)?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|e| DpackError::Manifest(e.to_string()))?;
    manifest.validate(limits.max_entries, limits.max_total_bytes)?;
    Ok(manifest)
}

/// Extrae las entradas del manifiesto. `resolve` decide donde escribir cada una (o que no hace falta);
/// `done` se llama cuando el contenido ya esta verificado (tamano y SHA-512 contra el manifiesto).
/// Lo que el manifiesto no lista se ignora. Cualquier discrepancia aborta: el que llama deshace.
pub fn extract(
    archive: &Path,
    manifest: &Manifest,
    limits: &Limits,
    resolve: &mut dyn FnMut(&ManifestEntry) -> Result<ExtractPlan, DpackError>,
    done: &mut Done<'_>,
    progress: &dyn Fn(usize, usize),
) -> Result<(), DpackError> {
    let mut reader = open(archive)?;
    check_headers(&reader, limits)?;

    let wanted: HashMap<&str, &ManifestEntry> = manifest.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let total = manifest.entries.len();
    let mut seen: HashSet<String> = HashSet::new();
    let mut written_total = 0u64;
    let mut failure: Option<DpackError> = None;

    let result = reader.for_each_entries(|entry, data| {
        if entry.is_directory() || entry.name() == MANIFEST_NAME {
            return Ok(true);
        }
        let Some(item) = wanted.get(entry.name()).copied() else { return Ok(true) };
        let outcome = (|| -> Result<(), DpackError> {
            if entry.has_windows_attributes && is_link(entry.windows_attributes()) {
                return Err(DpackError::Link(item.path.clone()));
            }
            if !seen.insert(item.path.clone()) {
                return Err(DpackError::Duplicate(item.path.clone()));
            }
            match resolve(item)? {
                ExtractPlan::Skip => done(item, None)?,
                ExtractPlan::WriteTo(dest) => {
                    write_verified(data, item, &dest, limits.max_total_bytes, &mut written_total)?;
                    done(item, Some(&dest))?;
                }
            }
            progress(seen.len(), total);
            Ok(())
        })();
        match outcome {
            Ok(()) => Ok(true),
            Err(e) => {
                failure = Some(e);
                Ok(false)
            }
        }
    });
    if let Some(e) = failure {
        return Err(e);
    }
    result?;

    if let Some(missing) = manifest.entries.iter().find(|e| !seen.contains(&e.path)) {
        return Err(DpackError::Missing(missing.path.clone()));
    }
    Ok(())
}

/// Escribe la entrada contando los bytes reales (no los declarados) y calculando su SHA-512 al vuelo.
fn write_verified(
    data: &mut dyn Read,
    item: &ManifestEntry,
    dest: &Path,
    max_total: u64,
    written_total: &mut u64,
) -> Result<(), DpackError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(dest)?;
    let mut hasher = Sha512Hasher::new();
    let mut buffer = vec![0u8; COPY_BUFFER];
    let mut written = 0u64;
    loop {
        let read = data.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        written += read as u64;
        *written_total += read as u64;
        if written > item.size || *written_total > max_total {
            drop(file);
            let _ = fs::remove_file(dest);
            return Err(DpackError::TooLarge);
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])?;
    }
    file.flush()?;
    drop(file);
    if written != item.size || hasher.finalize_hex() != item.sha512 {
        let _ = fs::remove_file(dest);
        return Err(DpackError::Mismatch(item.path.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{export, EntrySource, ExportRequest};

    fn game_with_files() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for (rel, data) in [
            ("mods/sodium.jar", vec![1u8; 5000]),
            ("mods/custom.jar.disabled", vec![2u8; 300]),
            ("config/sodium.toml", b"quality = 1\n".repeat(100)),
            ("config/vacio.cfg", Vec::new()),
            ("kubejs/server_scripts/a.js", b"console.log(1)".to_vec()),
            ("resourcepacks/pack.zip", vec![3u8; 2000]),
            ("saves/Mundo/level.dat", vec![9u8; 100]),
            ("logs/latest.log", b"secreto".to_vec()),
            ("options.txt", b"fov:90".to_vec()),
            ("mods/a.jar.part", b"a medias".to_vec()),
            ("resourcepacks/DisorderLauncher-skin.zip", b"skin".to_vec()),
        ] {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, data).unwrap();
        }
        dir
    }

    fn request<'a>(game: &'a Path, worlds: bool) -> ExportRequest<'a> {
        ExportRequest {
            name: "Mi pack",
            minecraft_version: "1.21.1",
            loader_kind: "fabric",
            loader_version: Some("0.16.10"),
            game_dir: game,
            include_worlds: worlds,
        }
    }

    fn export_to(game: &Path, worlds: bool) -> (tempfile::TempDir, PathBuf) {
        let out = tempfile::tempdir().unwrap();
        let dest = out.path().join("pack.7z");
        export(&request(game, worlds), &dest, &|_, _| {}).unwrap();
        (out, dest)
    }

    fn import_all(archive: &Path, to: &Path) -> Result<Manifest, DpackError> {
        let limits = Limits::default();
        let manifest = read_manifest(archive, &limits)?;
        extract(
            archive,
            &manifest,
            &limits,
            &mut |e| Ok(ExtractPlan::WriteTo(to.join(&e.path))),
            &mut |_, _| Ok(()),
            &|_, _| {},
        )?;
        Ok(manifest)
    }

    #[test]
    fn roundtrip_keeps_content_and_leaves_personal_state_out() {
        let game = game_with_files();
        let (_out, archive) = export_to(game.path(), false);
        let target = tempfile::tempdir().unwrap();
        let manifest = import_all(&archive, target.path()).unwrap();

        assert_eq!(manifest.pack.name, "Mi pack");
        assert_eq!((manifest.minecraft.version.as_str(), manifest.loader.kind.as_str()), ("1.21.1", "fabric"));
        let paths: Vec<&str> = manifest.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            ["config/sodium.toml", "config/vacio.cfg", "kubejs/server_scripts/a.js", "mods/custom.jar.disabled", "mods/sodium.jar", "resourcepacks/pack.zip"]
        );
        assert!(manifest.entries.iter().all(|e| e.source == EntrySource::Local));
        for entry in &manifest.entries {
            let original = fs::read(game.path().join(&entry.path)).unwrap();
            assert_eq!(fs::read(target.path().join(&entry.path)).unwrap(), original, "{}", entry.path);
        }
        for absent in ["saves", "logs", "options.txt"] {
            assert!(!target.path().join(absent).exists(), "{absent}");
        }
        assert!(!target.path().join("mods/a.jar.part").exists());
        assert!(!target.path().join("resourcepacks/DisorderLauncher-skin.zip").exists());
    }

    #[test]
    fn worlds_travel_only_when_asked() {
        let game = game_with_files();
        let (_o1, with) = export_to(game.path(), true);
        let target = tempfile::tempdir().unwrap();
        import_all(&with, target.path()).unwrap();
        assert_eq!(fs::read(target.path().join("saves/Mundo/level.dat")).unwrap(), vec![9u8; 100]);
    }

    #[test]
    fn export_is_a_standard_7z_without_leaking_account_data() {
        let game = game_with_files();
        let (_out, archive) = export_to(game.path(), false);
        let bytes = fs::read(&archive).unwrap();
        assert_eq!(&bytes[..6], &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C], "firma 7z");
        let manifest = read_manifest(&archive, &Limits::default()).unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        for forbidden in ["profile", "token", "accounts", "C:\\", "/Users/"] {
            assert!(!json.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn skipped_entries_are_not_written() {
        let game = game_with_files();
        let (_out, archive) = export_to(game.path(), false);
        let target = tempfile::tempdir().unwrap();
        let limits = Limits::default();
        let manifest = read_manifest(&archive, &limits).unwrap();
        let mut skipped = Vec::new();
        extract(
            &archive,
            &manifest,
            &limits,
            &mut |e| {
                Ok(if e.path.starts_with("mods/") { ExtractPlan::Skip } else { ExtractPlan::WriteTo(target.path().join(&e.path)) })
            },
            &mut |e, path| {
                if path.is_none() {
                    skipped.push(e.path.clone());
                }
                Ok(())
            },
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(skipped, ["mods/custom.jar.disabled", "mods/sodium.jar"]);
        assert!(!target.path().join("mods").exists());
        assert!(target.path().join("config/sodium.toml").is_file());
    }

    #[test]
    fn non_dpack_files_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let junk = dir.path().join("x.7z");
        fs::write(&junk, b"esto no es un 7z").unwrap();
        assert!(matches!(read_manifest(&junk, &Limits::default()), Err(DpackError::NotADpack)));
        assert!(read_manifest(&dir.path().join("no-existe.7z"), &Limits::default()).is_err());
    }

    /// Construye un `.7z` a mano para probar paquetes hostiles.
    fn hostile(files: &[(&str, &[u8])], manifest: Option<&str>) -> (tempfile::TempDir, PathBuf) {
        use sevenz_rust2::{ArchiveEntry, ArchiveWriter};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hostil.7z");
        let mut writer = ArchiveWriter::create(&path).unwrap();
        if let Some(json) = manifest {
            writer.push_archive_entry(ArchiveEntry::new_file(MANIFEST_NAME), Some(json.as_bytes())).unwrap();
        }
        for (name, data) in files {
            writer.push_archive_entry(ArchiveEntry::new_file(name), Some(*data)).unwrap();
        }
        writer.finish().unwrap();
        (dir, path)
    }

    fn manifest_json(entries: &str) -> String {
        format!(
            r#"{{"format_version":1,"pack":{{"name":"x"}},"minecraft":{{"version":"1.21"}},"loader":{{"type":"vanilla"}},"entries":[{entries}]}}"#
        )
    }

    fn entry_json(path: &str, data: &[u8]) -> String {
        let (hash, size) = artifact_store::hash_reader(data).unwrap();
        format!(r#"{{"path":"{path}","size":{size},"sha512":"{hash}","source":{{"type":"local"}}}}"#)
    }

    #[test]
    fn traversal_and_reserved_paths_in_the_manifest_are_rejected() {
        for bad in ["../fuera.jar", "/abs.jar", "mods/../../x", ".disorder/mods.json", "mods/con.jar", "C:/x"] {
            let (_d, archive) = hostile(&[], Some(&manifest_json(&entry_json(bad, b"x"))));
            assert!(matches!(read_manifest(&archive, &Limits::default()), Err(DpackError::UnsafePath(_))), "{bad}");
        }
    }

    #[test]
    fn duplicate_and_case_colliding_entries_are_rejected() {
        let entries = format!("{},{}", entry_json("mods/a.jar", b"x"), entry_json("mods/A.jar", b"y"));
        let (_d, archive) = hostile(&[], Some(&manifest_json(&entries)));
        assert!(matches!(read_manifest(&archive, &Limits::default()), Err(DpackError::Duplicate(_))));
    }

    #[test]
    fn future_format_versions_ask_for_an_update() {
        let json = manifest_json("").replace("\"format_version\":1", "\"format_version\":7");
        let (_d, archive) = hostile(&[], Some(&json));
        assert!(matches!(read_manifest(&archive, &Limits::default()), Err(DpackError::UnsupportedVersion(7))));
    }

    #[test]
    fn missing_manifest_is_not_a_dpack() {
        let (_d, archive) = hostile(&[("mods/a.jar", b"x")], None);
        assert!(matches!(read_manifest(&archive, &Limits::default()), Err(DpackError::NotADpack)));
    }

    #[test]
    fn tampered_content_fails_verification_and_leaves_no_file() {
        let manifest = manifest_json(&entry_json("mods/a.jar", b"original"));
        let (_d, archive) = hostile(&[("mods/a.jar", b"0riginal")], Some(&manifest));
        let target = tempfile::tempdir().unwrap();
        let result = import_all(&archive, target.path());
        assert!(matches!(result, Err(DpackError::Mismatch(_))), "{result:?}");
        assert!(!target.path().join("mods/a.jar").exists());
    }

    #[test]
    fn entries_missing_from_the_archive_are_detected() {
        let manifest = manifest_json(&entry_json("mods/a.jar", b"x"));
        let (_d, archive) = hostile(&[], Some(&manifest));
        let target = tempfile::tempdir().unwrap();
        assert!(matches!(import_all(&archive, target.path()), Err(DpackError::Missing(_))));
    }

    #[test]
    fn unlisted_entries_are_ignored_not_extracted() {
        let manifest = manifest_json(&entry_json("mods/a.jar", b"x"));
        let (_d, archive) = hostile(&[("mods/a.jar", b"x"), ("../evil.txt", b"mal"), ("extra.txt", b"y")], Some(&manifest));
        let target = tempfile::tempdir().unwrap();
        import_all(&archive, target.path()).unwrap();
        assert!(target.path().join("mods/a.jar").is_file());
        assert!(!target.path().join("extra.txt").exists());
        assert!(!target.path().parent().unwrap().join("evil.txt").exists());
    }

    #[test]
    fn size_and_entry_limits_apply() {
        let manifest = manifest_json(&entry_json("mods/a.jar", &[0u8; 2048]));
        let (_d, archive) = hostile(&[("mods/a.jar", &[0u8; 2048])], Some(&manifest));
        let tiny = Limits { max_entries: 10, max_total_bytes: 1024 };
        assert!(matches!(read_manifest(&archive, &tiny), Err(DpackError::TooLarge)));
        let few = Limits { max_entries: 0, max_total_bytes: 1 << 30 };
        assert!(matches!(read_manifest(&archive, &few), Err(DpackError::TooManyEntries)));
    }

    #[test]
    fn an_entry_bigger_than_declared_is_cut_off() {
        // El manifiesto declara 1 byte pero el archivo trae mucho mas: nunca se escribe completo.
        let (hash, _) = artifact_store::hash_reader(&b"x"[..]).unwrap();
        let manifest = manifest_json(&format!(r#"{{"path":"mods/a.jar","size":1,"sha512":"{hash}","source":{{"type":"local"}}}}"#));
        let (_d, archive) = hostile(&[("mods/a.jar", &[7u8; 4096])], Some(&manifest));
        let target = tempfile::tempdir().unwrap();
        assert!(import_all(&archive, target.path()).is_err());
        assert!(!target.path().join("mods/a.jar").exists());
    }
}
