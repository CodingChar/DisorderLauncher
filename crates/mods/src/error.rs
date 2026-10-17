#[derive(Debug, thiserror::Error)]
pub enum ModError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("download error: {0}")]
    Download(#[from] download_manager::DownloadError),
    #[error("nombre de archivo de mod invalido")]
    InvalidFileName,
    #[error("el archivo no es un mod valido para este loader")]
    NotAMod,
    #[error("{0} no es compatible con tu version de Minecraft")]
    NotCompatible(String),
    #[error("mod no encontrado")]
    NotFound,
    #[error("Necesita {0}, que no esta disponible para tu version")]
    MissingDependency(String),
    #[error("origen de descarga no permitido")]
    UntrustedSource,
    #[error("la descarga no trae un archivo utilizable")]
    NoFile,
    #[error("no hay una copia anterior para restaurar")]
    NothingToRollback,
}
