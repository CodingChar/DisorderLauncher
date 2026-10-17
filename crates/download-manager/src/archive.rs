//! Extraccion segura de ZIP. Cualquier entrada que intente salir del directorio de
//! destino (ZIP traversal / zip slip) hace fallar toda la extraccion.

use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Tope por defecto de bytes descomprimidos (proteccion frente a zip bombs).
pub const DEFAULT_MAX_UNCOMPRESSED: u64 = 1024 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("entrada de archivo insegura: {0}")]
    UnsafeEntry(String),
    #[error("el archivo descomprimido supera el limite permitido")]
    TooLarge,
}

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    /// Prefijos de entrada que se omiten (p. ej. `META-INF/`).
    pub exclude_prefixes: Vec<String>,
    pub max_uncompressed_bytes: u64,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self { exclude_prefixes: Vec::new(), max_uncompressed_bytes: DEFAULT_MAX_UNCOMPRESSED }
    }
}

/// Extrae `archive` en `dest` y devuelve cuantos archivos escribio. Bloqueante:
/// llamar desde `spawn_blocking` en contextos async.
pub fn extract_zip(
    archive: &Path,
    dest: &Path,
    options: &ExtractOptions,
) -> Result<usize, ArchiveError> {
    let file = fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    fs::create_dir_all(dest)?;

    let mut written = 0usize;
    let mut budget = options.max_uncompressed_bytes;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;

        // `enclosed_name` rechaza rutas absolutas y componentes `..`.
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| ArchiveError::UnsafeEntry(entry.name().to_string()))?;

        let name = relative.to_string_lossy().replace('\\', "/");
        if options.exclude_prefixes.iter().any(|p| name.starts_with(p.as_str())) {
            continue;
        }
        // Enlaces simbolicos: nunca se materializan.
        if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            continue;
        }

        let target = dest.join(&relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = fs::File::create(&target)?;
        let copied = io::copy(&mut (&mut entry).take(budget.saturating_add(1)), &mut out)?;
        if copied > budget {
            drop(out);
            let _ = fs::remove_file(&target);
            return Err(ArchiveError::TooLarge);
        }
        budget -= copied;

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o777));
        }
        written += 1;
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            writer.start_file(*name, opts).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn extracts_files_and_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        build_zip(&zip_path, &[("lwjgl.dll", b"dll"), ("sub/dir/x.txt", b"x")]);

        let dest = dir.path().join("out");
        let n = extract_zip(&zip_path, &dest, &ExtractOptions::default()).unwrap();
        assert_eq!(n, 2);
        assert_eq!(fs::read(dest.join("sub/dir/x.txt")).unwrap(), b"x");
    }

    #[test]
    fn excluded_prefixes_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("a.zip");
        build_zip(&zip_path, &[("META-INF/MANIFEST.MF", b"m"), ("keep.dll", b"k")]);

        let dest = dir.path().join("out");
        let options = ExtractOptions { exclude_prefixes: vec!["META-INF/".into()], ..Default::default() };
        extract_zip(&zip_path, &dest, &options).unwrap();
        assert!(!dest.join("META-INF").exists());
        assert!(dest.join("keep.dll").is_file());
    }

    #[test]
    fn traversal_entries_are_rejected_and_nothing_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        build_zip(&zip_path, &[("../escaped.txt", b"pwn")]);

        let dest = dir.path().join("out");
        let err = extract_zip(&zip_path, &dest, &ExtractOptions::default()).unwrap_err();
        assert!(matches!(err, ArchiveError::UnsafeEntry(_)));
        assert!(!dir.path().join("escaped.txt").exists());
    }

    #[test]
    fn absolute_paths_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        build_zip(&zip_path, &[("/abs.txt", b"pwn")]);
        let err = extract_zip(&zip_path, &dir.path().join("out"), &ExtractOptions::default()).unwrap_err();
        assert!(matches!(err, ArchiveError::UnsafeEntry(_)));
    }

    #[test]
    fn size_limit_is_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("big.zip");
        build_zip(&zip_path, &[("big.bin", &vec![0u8; 4096])]);
        let options = ExtractOptions { max_uncompressed_bytes: 100, ..Default::default() };
        let err = extract_zip(&zip_path, &dir.path().join("out"), &options).unwrap_err();
        assert!(matches!(err, ArchiveError::TooLarge));
    }
}
