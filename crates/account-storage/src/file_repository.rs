use crate::{AccountRepository, StorageError};
use identity_api::AccountRecord;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct FileAccountRepository {
    dir: PathBuf,
}

impl FileAccountRepository {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    fn account_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", id))
    }

    fn default_path(&self) -> PathBuf {
        self.dir.join("_default.txt")
    }

    async fn ensure_dir(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.dir).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl AccountRepository for FileAccountRepository {
    async fn list(&self) -> Result<Vec<AccountRecord>, StorageError> {
        self.ensure_dir().await?;
        let mut entries = fs::read_dir(&self.dir).await?;
        let mut records = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let data = fs::read_to_string(&path).await?;
                let record: AccountRecord = serde_json::from_str(&data)?;
                records.push(record);
            }
        }
        records.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(records)
    }

    async fn get(&self, id: &str) -> Result<Option<AccountRecord>, StorageError> {
        let path = self.account_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(&path).await?;
        Ok(Some(serde_json::from_str(&data)?))
    }

    async fn save(&self, record: &AccountRecord) -> Result<(), StorageError> {
        self.ensure_dir().await?;
        let path = self.account_path(&record.id);
        let part = path.with_extension("json.part");
        let data = serde_json::to_string_pretty(record)?;
        fs::write(&part, &data).await?;
        fs::rename(&part, &path).await?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        let path = self.account_path(id);
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        let default = self.get_default().await?;
        if default.as_deref() == Some(id) {
            let dp = self.default_path();
            if dp.exists() {
                fs::remove_file(&dp).await?;
            }
        }
        Ok(())
    }

    async fn get_default(&self) -> Result<Option<String>, StorageError> {
        let path = self.default_path();
        if !path.exists() {
            return Ok(None);
        }
        let id = fs::read_to_string(&path).await?.trim().to_string();
        if id.is_empty() {
            return Ok(None);
        }
        Ok(Some(id))
    }

    async fn set_default(&self, id: &str) -> Result<(), StorageError> {
        self.ensure_dir().await?;
        if self.get(id).await?.is_none() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        fs::write(self.default_path(), id).await?;
        Ok(())
    }
}
