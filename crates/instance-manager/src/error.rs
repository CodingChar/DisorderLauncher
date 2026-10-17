#[derive(Debug, thiserror::Error)]
pub enum InstanceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("instancia no encontrada: {0}")]
    NotFound(String),
    #[error("identificador de instancia invalido")]
    InvalidId,
    #[error("datos de instancia invalidos: {0}")]
    Invalid(String),
    #[error("la instancia fue creada con una version mas nueva del launcher (schema {0})")]
    UnsupportedSchema(u32),
}
