#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error("no compatible java found for version {0}")]
    NoCompatibleVersion(u32),
    #[error("java detection failed: {0}")]
    DetectionFailed(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("no adoptium release found for java {0}")]
    NoAdoptiumRelease(u32),
    #[error("extraction failed: {0}")]
    ExtractionFailed(String),
}
