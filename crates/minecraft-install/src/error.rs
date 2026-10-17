#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("meta error: {0}")]
    Meta(#[from] minecraft_meta::MetaError),
    #[error("download error: {0}")]
    Download(#[from] download_manager::DownloadError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no client download in version descriptor")]
    NoClientDownload,
    #[error("download failures: {0} files failed")]
    DownloadFailures(usize),
}
