#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("download error: {0}")]
    Download(#[from] download_manager::DownloadError),
    #[error("este loader no esta disponible para Minecraft {0}")]
    NoVersionFor(String),
    #[error("version o identificador invalido: {0}")]
    InvalidComponent(String),
    #[error("el perfil del loader no corresponde a la version pedida")]
    ProfileMismatch,
    #[error("origen de descarga no permitido: {0}")]
    UntrustedSource(String),
    #[error("{0} archivos del loader no se pudieron descargar")]
    DownloadFailures(usize),
}
