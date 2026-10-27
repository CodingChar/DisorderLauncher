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
        check_host_in(self.allowed_host_suffixes.as_deref(), url)
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
        let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
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
                progress(DownloadProgress {
                    completed: completed.load(Ordering::Relaxed),
                    total,
                    bytes_downloaded: bytes_downloaded.load(Ordering::Relaxed),
                    bytes_total,
                    current_file: task.dest.display().to_string(),
                });
                continue;
            }
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let http = self.http.clone();
            let allowed = self.allowed_host_suffixes.clone();
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
                    download_with_retry(&http, allowed.as_deref(), &task, MAX_RETRIES, &bytes_downloaded).await;

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
        if let Ok(mut file) = std::fs::File::open(&task.dest) {
            let mut hasher = Sha1::new();
            if std::io::copy(&mut file, &mut hasher).is_err() {
                return false;
            }
            let actual = format!("{:x}", hasher.finalize());
            return actual == *expected_sha1;
        }
    }
    false
}

/// Comprueba el host contra la lista de permitidos (si la hay). Se usa con la URL pedida y con la URL final tras redirecciones.
fn check_host_in(allowed: Option<&[String]>, url: &str) -> Result<(), DownloadError> {
    let Some(allowed) = allowed else { return Ok(()) };
    let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(str::to_lowercase));
    let permitted = host
        .as_deref()
        .is_some_and(|h| allowed.iter().any(|suffix| h == suffix || h.ends_with(&format!(".{suffix}"))));
    if permitted { Ok(()) } else { Err(DownloadError::UntrustedHost(url.chars().take(80).collect())) }
}

/// La URL final (tras redirecciones) debe seguir siendo https y, si hay lista, de un host permitido.
fn check_final_url(allowed: Option<&[String]>, final_url: &url::Url) -> Result<(), DownloadError> {
    if final_url.scheme() != "https" {
        return Err(DownloadError::InsecureUrl(final_url.as_str().chars().take(80).collect()));
    }
    check_host_in(allowed, final_url.as_str())
}

async fn download_with_retry(
    http: &reqwest::Client,
    allowed: Option<&[String]>,
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

        match download_single(http, allowed, task, bytes_counter).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = e.to_string();
                if matches!(e, DownloadError::HashMismatch { .. } | DownloadError::InsecureUrl(_) | DownloadError::UntrustedHost(_))
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
    allowed: Option<&[String]>,
    task: &DownloadTask,
    bytes_counter: &AtomicU64,
) -> Result<(), DownloadError> {
    if let Some(parent) = task.dest.parent() {
        fs::create_dir_all(parent).await?;
    }

    let response = http.get(&task.url).send().await?.error_for_status()?;
    // Una redireccion no puede sacar la descarga de los hosts de confianza ni degradarla a http.
    check_final_url(allowed, response.url())?;
    save_response(response, task, bytes_counter).await
}

async fn save_response(
    response: reqwest::Response,
    task: &DownloadTask,
    bytes_counter: &AtomicU64,
) -> Result<(), DownloadError> {
    // Dos instancias pueden instalar el mismo asset a la vez: cada descarga
    // necesita su propio temporal, que tambien se elimina al cancelar la tarea.
    let part_path = part_path(&task.dest)?;
    let result = async {
        let mut response = response;
        let mut file = fs::File::create(&part_path).await?;
        let mut hasher = Sha1::new();
        while let Some(chunk) = response.chunk().await? {
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
            bytes_counter.fetch_add(chunk.len() as u64, Ordering::Relaxed);
        }
        file.flush().await?;
        drop(file);

        if let Some(expected) = &task.sha1 {
            let actual = format!("{:x}", hasher.finalize());
            if actual != *expected {
                return Err(DownloadError::HashMismatch {
                    path: task.dest.display().to_string(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        fs_safe::rename(&part_path, &task.dest).await?;
        Ok(())
    }.await;
    if result.is_err() {
        let _ = fs::remove_file(&part_path).await;
    }
    result
}

fn part_path(dest: &Path) -> std::io::Result<tempfile::TempPath> {
    let mut name = dest
        .file_name()
        .unwrap_or_default()
        .to_os_string();
    name.push(".part-");
    tempfile::Builder::new()
        .prefix(&name)
        .tempfile_in(dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new(".")))
        .map(|file| file.into_temp_path())
}

#[cfg(test)]
fn hex_sha1(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Solo el transporte del fixture usa HTTP local. La entrada de produccion
    // sigue exigiendo HTTPS y comprueba el destino antes de escribir.
    async fn response(body: &[u8], declared_length: usize) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_vec();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let (mut stream, _) = listener.accept().await.unwrap();
            {
                let mut reader = tokio::io::BufReader::new(&mut stream);
                let mut header = String::new();
                loop {
                    header.clear();
                    let amount = reader.read_line(&mut header).await.unwrap();
                    if amount == 0 || header == "\r\n" { break; }
                }
            }
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {declared_length}\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
            for chunk in body.chunks(8192) {
                stream.write_all(chunk).await.unwrap();
            }
        });
        reqwest::Client::new().get(format!("http://{address}/file")).send().await.unwrap()
    }

    #[tokio::test]
    async fn streams_and_verifies_before_replacing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let body = vec![42; 1024 * 1024];
        let task = DownloadTask {
            url: "https://example.com/file".into(), dest: dir.path().join("file.jar"),
            sha1: Some(hex_sha1(&body)), size: Some(body.len() as u64),
        };
        std::fs::write(&task.dest, b"old file").unwrap();
        let count = AtomicU64::new(0);
        save_response(response(&body, body.len()).await, &task, &count).await.unwrap();
        assert_eq!(std::fs::read(&task.dest).unwrap(), body);
        assert_eq!(count.load(Ordering::Relaxed), body.len() as u64);
        assert!(should_skip(&task));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn invalid_or_interrupted_download_preserves_existing_file_and_removes_temp() {
        let dir = tempfile::tempdir().unwrap();
        let task = DownloadTask {
            url: "https://example.com/file".into(), dest: dir.path().join("file.jar"),
            sha1: Some(hex_sha1(b"expected")), size: None,
        };
        std::fs::write(&task.dest, b"old file").unwrap();
        for length in [3, 100] {
            let result = save_response(response(b"bad", length).await, &task, &AtomicU64::new(0)).await;
            assert!(result.is_err());
            if length == 3 { assert!(matches!(result, Err(DownloadError::HashMismatch { .. }))); }
            assert_eq!(std::fs::read(&task.dest).unwrap(), b"old file");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        }
    }

    #[test]
    fn final_url_after_redirect_is_rechecked() {
        let allowed = vec!["mojang.com".to_string()];
        let ok = url::Url::parse("https://piston-data.mojang.com/x").unwrap();
        assert!(check_final_url(Some(&allowed), &ok).is_ok());
        let evil = url::Url::parse("https://evil.example.com/x").unwrap();
        assert!(matches!(check_final_url(Some(&allowed), &evil), Err(DownloadError::UntrustedHost(_))));
        let plain = url::Url::parse("http://piston-data.mojang.com/x").unwrap();
        assert!(matches!(check_final_url(Some(&allowed), &plain), Err(DownloadError::InsecureUrl(_))));
        assert!(check_final_url(None, &evil).is_ok());
    }

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
    fn concurrent_downloads_use_distinct_temporary_files_and_clean_up() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("some.jar");
        let first = part_path(&dest).unwrap();
        let second = part_path(&dest).unwrap();
        assert_ne!(first.to_path_buf(), second.to_path_buf());
        assert_eq!(first.parent(), Some(dir.path()));
        let path = first.to_path_buf();
        assert!(path.exists());
        drop(first);
        assert!(!path.exists());
        assert!(second.exists());
    }
}
