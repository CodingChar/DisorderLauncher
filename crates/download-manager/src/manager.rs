use crate::{DownloadError, DownloadFailure, DownloadProgress, DownloadReport, DownloadTask};
use sha1::{Digest, Sha1};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

const MAX_RETRIES: u32 = 3;
const BACKOFF_BASE_MS: u64 = 500;

pub struct DownloadManager {
    http: reqwest::Client,
    /// Si se define, solo se descarga desde estos dominios (o sus subdominios).
    allowed_host_suffixes: Option<Vec<String>>,
}

impl DownloadManager {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http, allowed_host_suffixes: None }
    }

    /// Restringe las descargas a los dominios dados y a sus subdominios
    /// (`mojang.com` permite `piston-data.mojang.com`, no `evilmojang.com`).
    pub fn with_allowed_host_suffixes(mut self, suffixes: &[&str]) -> Self {
        self.allowed_host_suffixes = Some(suffixes.iter().map(|s| s.to_lowercase()).collect());
        self
    }

    fn check_host(&self, url: &str) -> Result<(), DownloadError> {
        let Some(allowed) = &self.allowed_host_suffixes else { return Ok(()) };
        let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_lowercase));
        let permitted = host
            .as_deref()
            .is_some_and(|h| allowed.iter().any(|suffix| h == suffix || h.ends_with(&format!(".{suffix}"))));
        if permitted { Ok(()) } else { Err(DownloadError::UntrustedHost(url.chars().take(80).collect())) }
    }

    pub async fn download_all<F>(
        &self,
        tasks: Vec<DownloadTask>,
        concurrency: usize,
        progress: F,
    ) -> Result<DownloadReport, DownloadError>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let total = tasks.len();
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let completed = Arc::new(AtomicUsize::new(0));
        let bytes_downloaded = Arc::new(AtomicU64::new(0));
        let bytes_total: u64 = tasks.iter().filter_map(|t| t.size).sum();
        let progress = Arc::new(progress);
        let failures = Arc::new(tokio::sync::Mutex::new(Vec::<DownloadFailure>::new()));
        let skipped = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::with_capacity(total);

        for task in tasks {
            // Origen no permitido: se rechaza sin abrir ninguna conexion.
            if let Err(e) = self.check_host(&task.url) {
                failures.lock().await.push(DownloadFailure {
                    url: task.url.clone(),
                    dest: task.dest.clone(),
                    error: e.to_string(),
                });
                completed.fetch_add(1, Ordering::Relaxed);
                continue;
            }
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let http = self.http.clone();
            let completed = completed.clone();
            let bytes_downloaded = bytes_downloaded.clone();
            let progress = progress.clone();
            let failures = failures.clone();
            let skipped = skipped.clone();

            let handle = tokio::spawn(async move {
                // El hash de archivos grandes no debe bloquear el runtime asincrono.
                let probe = task.clone();
                let already_valid = tokio::task::spawn_blocking(move || should_skip(&probe)).await.unwrap_or(false);
                if already_valid {
                    skipped.fetch_add(1, Ordering::Relaxed);
                    completed.fetch_add(1, Ordering::Relaxed);
                    progress(DownloadProgress {
                        completed: completed.load(Ordering::Relaxed),
                        total,
                        bytes_downloaded: bytes_downloaded.load(Ordering::Relaxed),
                        bytes_total,
                        current_file: task.dest.display().to_string(),
                    });
                    drop(permit);
                    return;
                }

                let result =
                    download_with_retry(&http, &task, MAX_RETRIES, &bytes_downloaded).await;

                match result {
                    Ok(()) => {}
                    Err(e) => {
                        failures.lock().await.push(DownloadFailure {
                            url: task.url.clone(),
                            dest: task.dest.clone(),
                            error: e.to_string(),
                        });
                    }
                }

                completed.fetch_add(1, Ordering::Relaxed);
                progress(DownloadProgress {
                    completed: completed.load(Ordering::Relaxed),
                    total,
                    bytes_downloaded: bytes_downloaded.load(Ordering::Relaxed),
                    bytes_total,
                    current_file: task.dest.display().to_string(),
                });

                drop(permit);
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        let failed = failures.lock().await.clone();
        let skipped_count = skipped.load(Ordering::Relaxed);

        Ok(DownloadReport {
            total,
            succeeded: total - failed.len() - skipped_count,
            skipped: skipped_count,
            failed,
        })
    }
}

fn should_skip(task: &DownloadTask) -> bool {
    if !task.dest.exists() {
        return false;
    }
    if let Some(expected_sha1) = &task.sha1 {
        if let Ok(data) = std::fs::read(&task.dest) {
            let actual = hex_sha1(&data);
            return actual == *expected_sha1;
        }
    }
    false
}

async fn download_with_retry(
    http: &reqwest::Client,
    task: &DownloadTask,
    max_retries: u32,
    bytes_counter: &AtomicU64,
) -> Result<(), DownloadError> {
    if !task.url.starts_with("https://") {
        return Err(DownloadError::InsecureUrl(task.url.clone()));
    }

    let mut last_error = String::new();

    for attempt in 0..=max_retries {
        if attempt > 0 {
            let delay = BACKOFF_BASE_MS * 2u64.pow(attempt - 1);
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        match download_single(http, task, bytes_counter).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = e.to_string();
                if matches!(e, DownloadError::HashMismatch { .. } | DownloadError::InsecureUrl(_))
                {
                    return Err(e);
                }
            }
        }
    }

    Err(DownloadError::RetriesExhausted {
        url: task.url.clone(),
        last_error,
    })
}

async fn download_single(
    http: &reqwest::Client,
    task: &DownloadTask,
    bytes_counter: &AtomicU64,
) -> Result<(), DownloadError> {
    if let Some(parent) = task.dest.parent() {
        fs::create_dir_all(parent).await?;
    }

    let part_path = part_path(&task.dest);
    let response = http.get(&task.url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;

    bytes_counter.fetch_add(bytes.len() as u64, Ordering::Relaxed);

    let mut file = fs::File::create(&part_path).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;
    drop(file);

    if let Some(expected) = &task.sha1 {
        let actual = hex_sha1(&bytes);
        if actual != *expected {
            let _ = fs::remove_file(&part_path).await;
            return Err(DownloadError::HashMismatch {
                path: task.dest.display().to_string(),
                expected: expected.clone(),
                actual,
            });
        }
    }

    fs::rename(&part_path, &task.dest).await?;
    Ok(())
}

fn part_path(dest: &Path) -> std::path::PathBuf {
    let mut name = dest
        .file_name()
        .unwrap_or_default()
        .to_os_string();
    name.push(".part");
    dest.with_file_name(name)
}

fn hex_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_allowlist_accepts_subdomains_but_not_lookalikes() {
        let m = DownloadManager::new(reqwest::Client::new()).with_allowed_host_suffixes(&["mojang.com", "minecraft.net"]);
        assert!(m.check_host("https://piston-data.mojang.com/v1/objects/a/client.jar").is_ok());
        assert!(m.check_host("https://libraries.minecraft.net/x.jar").is_ok());
        assert!(m.check_host("https://mojang.com/x").is_ok());
        for bad in ["https://evilmojang.com/x", "https://mojang.com.evil.io/x", "https://example.com/x", "not a url", "https://minecraft.net.attacker.com/x"] {
            assert!(m.check_host(bad).is_err(), "{bad}");
        }
        // Sin lista no se restringe (solo HTTPS en la descarga).
        assert!(DownloadManager::new(reqwest::Client::new()).check_host("https://example.com/x").is_ok());
    }

    #[test]
    fn hex_sha1_known_value() {
        let hash = hex_sha1(b"hello world");
        assert_eq!(hash, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn part_path_appends_extension() {
        let dest = std::path::PathBuf::from("/tmp/libs/some.jar");
        let part = part_path(&dest);
        assert_eq!(part, std::path::PathBuf::from("/tmp/libs/some.jar.part"));
    }
}
