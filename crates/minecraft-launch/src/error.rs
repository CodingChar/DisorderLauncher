use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("placeholder sin resolver en los argumentos de lanzamiento: {0}")]
    UnresolvedPlaceholder(String),

    #[error("la version no define argumentos de juego")]
    MissingGameArguments,

    #[error("falta Java en {0}")]
    JavaMissing(PathBuf),

    #[error("faltan {} archivos necesarios para jugar", .0.len())]
    MissingFiles(Vec<PathBuf>),

    #[error("el classpath contiene un separador invalido: {0}")]
    InvalidClasspath(String),

    #[error("no se pudo iniciar el proceso de Minecraft: {0}")]
    Spawn(#[source] std::io::Error),

    #[error("no se pudo preparar el directorio del juego: {0}")]
    WorkingDirectory(#[source] std::io::Error),
}
