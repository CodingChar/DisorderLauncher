use std::io;

#[derive(Debug, thiserror::Error)]
pub enum DpackError {
    #[error("error de disco: {0}")]
    Io(#[from] io::Error),
    #[error("no se pudo leer el paquete: {0}")]
    Archive(String),
    #[error("el archivo no es un paquete de instancia")]
    NotADpack,
    #[error("el paquete usa un formato mas nuevo (version {0})")]
    UnsupportedVersion(u32),
    #[error("ruta no permitida en el paquete: {0}")]
    UnsafePath(String),
    #[error("el paquete es demasiado grande")]
    TooLarge,
    #[error("el paquete tiene demasiados archivos")]
    TooManyEntries,
    #[error("el contenido de {0} no coincide con el paquete")]
    Mismatch(String),
    #[error("falta {0} en el paquete")]
    Missing(String),
    #[error("el paquete tiene entradas repetidas: {0}")]
    Duplicate(String),
    #[error("entrada no permitida (enlace simbolico): {0}")]
    Link(String),
    #[error("manifiesto no valido: {0}")]
    Manifest(String),
    #[error("{0}")]
    Store(#[from] artifact_store::ArtifactError),
}

impl From<sevenz_rust2::Error> for DpackError {
    fn from(e: sevenz_rust2::Error) -> Self {
        Self::Archive(e.to_string())
    }
}
