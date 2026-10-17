use crate::{AccountRepository, StorageError};
use identity_api::AccountRecord;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::fs;

/// Version actual del formato de archivo de cuenta (`schemaVersion`).
pub const ACCOUNT_SCHEMA_VERSION: u32 = 1;

/// Hechos de mantenimiento que la aplicacion puede contar al usuario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageNotice {
    /// Un archivo de formato antiguo se convirtio (queda una copia `.bak-v<N>`).
    Migrated { account: String, from_version: u32 },
    /// Un archivo ilegible se aparto como `.corrupt` para no bloquear el resto.
    Quarantined { file_name: String },
    /// Un archivo creado por una version mas nueva del launcher: no se toca ni se lee.
    NewerFormat { file_name: String },
}

pub struct FileAccountRepository {
    dir: PathBuf,
    notices: Mutex<Vec<StorageNotice>>,
}

impl FileAccountRepository {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self { dir: dir.as_ref().to_path_buf(), notices: Mutex::new(Vec::new()) }
    }

    /// Avisos acumulados desde la ultima llamada.
    pub fn take_notices(&self) -> Vec<StorageNotice> {
        std::mem::take(&mut *self.notices.lock().unwrap_or_else(|p| p.into_inner()))
    }

    fn notify(&self, notice: StorageNotice) {
        self.notices.lock().unwrap_or_else(|p| p.into_inner()).push(notice);
    }

    /// El id se usa como nombre de archivo: solo caracteres seguros, nunca separadores de ruta.
    fn account_path(&self, id: &str) -> Result<PathBuf, StorageError> {
        validate_id(id)?;
        Ok(self.dir.join(format!("{id}.json")))
    }

    fn default_path(&self) -> PathBuf {
        self.dir.join("_default.txt")
    }

    async fn ensure_dir(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.dir).await?;
        Ok(())
    }

    /// Lee un archivo de cuenta aplicando migraciones. `Ok(None)` si esta danado o es de una version futura.
    async fn read_file(&self, path: &Path) -> Result<Option<AccountRecord>, StorageError> {
        let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let data = fs::read_to_string(path).await?;

        let Ok(value) = serde_json::from_str::<Value>(&data) else {
            self.quarantine(path, &file_name).await;
            return Ok(None);
        };
        let version = value.get("schemaVersion").and_then(Value::as_u64).unwrap_or(0) as u32;
        if version > ACCOUNT_SCHEMA_VERSION {
            self.notify(StorageNotice::NewerFormat { file_name });
            return Ok(None);
        }

        let migrated = if version < ACCOUNT_SCHEMA_VERSION { migrate_to_current(value, version) } else { value };
        let record = match serde_json::from_value::<AccountRecord>(strip_schema(migrated)) {
            Ok(record) => record,
            Err(_) => {
                self.quarantine(path, &file_name).await;
                return Ok(None);
            }
        };

        if version < ACCOUNT_SCHEMA_VERSION {
            // Nunca se reescribe sin una copia del original.
            let backup = path.with_file_name(format!("{file_name}.bak-v{version}"));
            fs::copy(path, &backup).await?;
            self.write_record(&record).await?;
            self.notify(StorageNotice::Migrated { account: record.id.clone(), from_version: version });
        }
        Ok(Some(record))
    }

    async fn quarantine(&self, path: &Path, file_name: &str) {
        let target = path.with_file_name(format!("{file_name}.corrupt"));
        if fs::rename(path, &target).await.is_ok() {
            tracing::warn!(file = %file_name, "archivo de cuenta ilegible apartado como .corrupt");
            self.notify(StorageNotice::Quarantined { file_name: file_name.to_string() });
        }
    }

    async fn write_record(&self, record: &AccountRecord) -> Result<(), StorageError> {
        self.ensure_dir().await?;
        let path = self.account_path(&record.id)?;
        let part = path.with_extension("json.part");
        let mut value = serde_json::to_value(record)?;
        if let Some(object) = value.as_object_mut() {
            object.insert("schemaVersion".into(), ACCOUNT_SCHEMA_VERSION.into());
        }
        fs::write(&part, serde_json::to_string_pretty(&value)?).await?;
        fs::rename(&part, &path).await?;
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<(), StorageError> {
    let ok = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok { Ok(()) } else { Err(StorageError::InvalidId) }
}

fn strip_schema(mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.remove("schemaVersion");
    }
    value
}

/// Migraciones deterministas, una por version. Hoy: v0 (sin `schemaVersion`, claves en snake_case
/// y proveedor con mayuscula inicial) -> v1 (camelCase y proveedor en minuscula).
fn migrate_to_current(mut value: Value, from: u32) -> Value {
    if from == 0 {
        value = camel_case_keys(value);
        if let Some(provider) = value.get_mut("provider") {
            if let Some(name) = provider.as_str() {
                *provider = Value::String(name.to_lowercase());
            }
        }
    }
    value
}

fn camel_case_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.into_iter().map(|(k, v)| (camel_case(&k), camel_case_keys(v))).collect()),
        Value::Array(items) => Value::Array(items.into_iter().map(camel_case_keys).collect()),
        other => other,
    }
}

fn camel_case(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    let mut upper = false;
    for c in key.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[async_trait::async_trait]
impl AccountRepository for FileAccountRepository {
    async fn list(&self) -> Result<Vec<AccountRecord>, StorageError> {
        self.ensure_dir().await?;
        let mut entries = fs::read_dir(&self.dir).await?;
        let mut records = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            // `.json` exacto: las copias `.bak-vN`, `.corrupt` y `.part` quedan fuera.
            if path.extension().is_some_and(|ext| ext == "json") {
                // Un archivo danado no debe impedir ver el resto de perfiles.
                if let Some(record) = self.read_file(&path).await? {
                    records.push(record);
                }
            }
        }
        records.sort_by_key(|r| r.created_at);
        Ok(records)
    }

    async fn get(&self, id: &str) -> Result<Option<AccountRecord>, StorageError> {
        let Ok(path) = self.account_path(id) else { return Ok(None) };
        if !path.exists() {
            return Ok(None);
        }
        self.read_file(&path).await
    }

    async fn save(&self, record: &AccountRecord) -> Result<(), StorageError> {
        self.write_record(record).await
    }

    async fn delete(&self, id: &str) -> Result<(), StorageError> {
        let path = self.account_path(id)?;
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

    /// Un predeterminado que apunta a una cuenta inexistente se ignora.
    async fn get_default(&self) -> Result<Option<String>, StorageError> {
        let path = self.default_path();
        if !path.exists() {
            return Ok(None);
        }
        let id = fs::read_to_string(&path).await?.trim().to_string();
        if id.is_empty() || validate_id(&id).is_err() || !self.account_path(&id)?.exists() {
            return Ok(None);
        }
        Ok(Some(id))
    }

    async fn set_default(&self, id: &str) -> Result<(), StorageError> {
        self.ensure_dir().await?;
        if self.get(id).await?.is_none() {
            return Err(StorageError::NotFound(id.to_string()));
        }
        let part = self.default_path().with_extension("txt.part");
        fs::write(&part, id).await?;
        fs::rename(&part, self.default_path()).await?;
        Ok(())
    }
}
