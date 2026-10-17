#[derive(Debug, thiserror::Error)]
pub enum MetaError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("version not found: {0}")]
    VersionNotFound(String),
}
