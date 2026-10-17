#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("url must use https: {0}")]
    InsecureUrl(String),
    #[error("all retries exhausted for {url}: {last_error}")]
    RetriesExhausted { url: String, last_error: String },
}
