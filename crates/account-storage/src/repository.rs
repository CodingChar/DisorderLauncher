use identity_api::AccountRecord;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

#[async_trait::async_trait]
pub trait AccountRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<AccountRecord>, StorageError>;
    async fn get(&self, id: &str) -> Result<Option<AccountRecord>, StorageError>;
    async fn save(&self, record: &AccountRecord) -> Result<(), StorageError>;
    async fn delete(&self, id: &str) -> Result<(), StorageError>;
    async fn get_default(&self) -> Result<Option<String>, StorageError>;
    async fn set_default(&self, id: &str) -> Result<(), StorageError>;
}
