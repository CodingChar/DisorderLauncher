use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("error de disco: {0}")]
    Io(#[from] io::Error),
    #[error("identificador de contenido no valido")]
    InvalidHash,
    #[error("el archivo no esta en el almacen")]
    Missing,
    #[error("no se puede leer el registro de archivos de una instancia")]
    UnreadableIndex,
    #[error("el contenido no coincide con lo esperado")]
    Mismatch,
}
