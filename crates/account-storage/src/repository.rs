use identity_api::AccountRecord;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid account id")]
    InvalidId,
}

#[async_trait::async_trait]
pub trait AccountRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<AccountRecord>, StorageError>;
    async fn get(&self, id: &str) -> Result<Option<AccountRecord>, StorageError>;
    async fn save(&self, record: &AccountRecord) -> Result<(), StorageError>;
    /// Anota el ultimo uso releyendo el perfil actual, sin pisar cambios hechos mientras tanto (p. ej. la skin).
    async fn touch_used(&self, id: &str, at: chrono::DateTime<chrono::Utc>) -> Result<(), StorageError>;
    async fn delete(&self, id: &str) -> Result<(), StorageError>;
    async fn get_default(&self) -> Result<Option<String>, StorageError>;
    async fn set_default(&self, id: &str) -> Result<(), StorageError>;
}
