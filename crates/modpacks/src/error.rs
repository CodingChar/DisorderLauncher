#[derive(Debug, thiserror::Error)]
pub enum ModpackError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("archive error: {0}")]
    Archive(#[from] download_manager::archive::ArchiveError),
    #[error("download error: {0}")]
    Download(#[from] download_manager::DownloadError),
    #[error("mods error: {0}")]
    Mods(#[from] mods::ModError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("el archivo no es un modpack de Modrinth valido")]
    NotAModpack,
    #[error("el archivo es un modpack de {0}, no de Modrinth")]
    OtherLauncher(String),
    #[error("formato de modpack no soportado: {0}")]
    UnsupportedFormat(String),
    #[error("el modpack contiene una ruta insegura: {0}")]
    UnsafePath(String),
    #[error("el modpack descarga desde un origen no permitido: {0}")]
    UntrustedSource(String),
    #[error("el modpack es demasiado grande")]
    TooLarge,
    #[error("falta el hash de {0}")]
    MissingHash(String),
    #[error("este modpack necesita {0}, que aun no esta disponible")]
    UnsupportedLoader(String),
    #[error("{0} archivos del modpack no se pudieron descargar")]
    DownloadFailures(usize),
}
