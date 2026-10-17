#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("store unavailable: {0}")]
    StoreUnavailable(String),
    #[error("key not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait CredentialStore: Send + Sync {
    async fn put_secret(&self, key: &str, secret: &str) -> Result<(), CredentialError>;
    async fn get_secret(&self, key: &str) -> Result<Option<String>, CredentialError>;
    async fn delete_secret(&self, key: &str) -> Result<(), CredentialError>;
}

pub struct InMemoryCredentialStore {
    secrets: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CredentialStore for InMemoryCredentialStore {
    async fn put_secret(&self, key: &str, secret: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .map_err(|e| CredentialError::Other(e.to_string()))?
            .insert(key.to_string(), secret.to_string());
        Ok(())
    }

    async fn get_secret(&self, key: &str) -> Result<Option<String>, CredentialError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|e| CredentialError::Other(e.to_string()))?
            .get(key)
            .cloned())
    }

    async fn delete_secret(&self, key: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .map_err(|e| CredentialError::Other(e.to_string()))?
            .remove(key);
        Ok(())
    }
}
