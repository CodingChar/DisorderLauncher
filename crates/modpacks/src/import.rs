use crate::{ModpackError, ModpackIndex};
use download_manager::archive::{extract_zip_prefixed, ExtractOptions};
use download_manager::{DownloadManager, DownloadTask};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Descarga los archivos del modpack en la carpeta del juego, verificando SHA-1.
/// Si una URL falla se prueba la siguiente alternativa del archivo.
pub async fn download_pack_files(
    http: &reqwest::Client,
    index: &ModpackIndex,
    game_dir: &Path,
    concurrency: usize,
    progress: Arc<dyn Fn(usize, usize) + Send + Sync>,
) -> Result<usize, ModpackError> {
    let files: Vec<_> = index.client_files().collect();
    let total = files.len();
    let manager = DownloadManager::new(http.clone());

    // Indices de los archivos que aun faltan por descargar y la URL alternativa a usar.
    let mut pending: Vec<usize> = (0..total).collect();
    let max_alternatives = files.iter().map(|f| f.downloads.len()).max().unwrap_or(0);

    for attempt in 0..max_alternatives {
        if pending.is_empty() {
            break;
        }
        let tasks: Vec<DownloadTask> = pending
            .iter()
            .filter_map(|&i| {
                let f = files[i];
                f.downloads.get(attempt).map(|url| DownloadTask {
                    url: url.clone(),
                    dest: game_dir.join(&f.path),
                    size: (f.file_size > 0).then_some(f.file_size),
                    sha1: Some(f.hashes.sha1.to_lowercase()),
                })
            })
            .collect();

        let done_before = total - pending.len();
        let cb = progress.clone();
        let report = manager
            .download_all(tasks, concurrency.max(1), move |p| cb(done_before + p.completed, total))
            .await?;

        let failed: std::collections::HashSet<PathBuf> = report.failed.iter().map(|f| f.dest.clone()).collect();
        pending.retain(|&i| failed.contains(&game_dir.join(&files[i].path)));
    }

    if pending.is_empty() {
        Ok(total)
    } else {
        Err(ModpackError::DownloadFailures(pending.len()))
    }
}

/// Aplica `overrides/` y luego `client-overrides/` sobre la carpeta del juego.
/// Devuelve cuantos archivos se escribieron.
pub async fn apply_overrides(mrpack: &Path, game_dir: &Path) -> Result<usize, ModpackError> {
    let mrpack = mrpack.to_path_buf();
    let game_dir = game_dir.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<usize, ModpackError> {
        let options = ExtractOptions { exclude_prefixes: vec![".disorder".to_string()], ..Default::default() };
        let mut written = 0;
        for prefix in ["overrides/", "client-overrides/"] {
            written += extract_zip_prefixed(&mrpack, prefix, &game_dir, &options)?;
        }
        Ok(written)
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn overrides_are_applied_in_order_and_protect_internal_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pack = dir.path().join("p.mrpack");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&pack).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        for (name, data) in [
            ("modrinth.index.json", &b"{}"[..]),
            ("overrides/config/a.toml", b"base"),
            ("overrides/options.txt", b"o"),
            ("client-overrides/config/a.toml", b"client"),
            ("overrides/.disorder/mods.json", b"evil"),
        ] {
            w.start_file(name, o).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();

        let game = dir.path().join("game");
        let n = apply_overrides(&pack, &game).await.unwrap();
        assert_eq!(n, 3);
        assert_eq!(std::fs::read(game.join("config/a.toml")).unwrap(), b"client", "client-overrides gana");
        assert!(!game.join(".disorder").exists(), "no se puede pisar el estado interno del launcher");
    }
}
