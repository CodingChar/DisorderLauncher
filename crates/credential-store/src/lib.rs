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

pub struct KeyringCredentialStore {
    service: String,
}

impl KeyringCredentialStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, CredentialError> {
        keyring::Entry::new(&self.service, key)
            .map_err(|e| CredentialError::StoreUnavailable(e.to_string()))
    }
}

#[async_trait::async_trait]
impl CredentialStore for KeyringCredentialStore {
    async fn put_secret(&self, key: &str, secret: &str) -> Result<(), CredentialError> {
        let entry = self.entry(key)?;
        entry
            .set_password(secret)
            .map_err(|e| CredentialError::StoreUnavailable(e.to_string()))
    }

    async fn get_secret(&self, key: &str) -> Result<Option<String>, CredentialError> {
        let entry = self.entry(key)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CredentialError::StoreUnavailable(e.to_string())),
        }
    }

    async fn delete_secret(&self, key: &str) -> Result<(), CredentialError> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CredentialError::StoreUnavailable(e.to_string())),
        }
    }
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
