use crate::ModError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;

/// Registro de mods instalados desde Modrinth: permite saber de donde vino cada archivo
/// (para actualizar) y conservar la version anterior (para volver atras).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModLock {
    pub schema_version: u32,
    pub mods: Vec<LockedMod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockedMod {
    /// Nombre del archivo activo, sin el sufijo `.disabled`.
    pub file: String,
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub title: String,
    pub sha1: String,
    pub installed_at: DateTime<Utc>,
    /// Proyecto que pidio este mod como dependencia (si no lo eligio el usuario).
    #[serde(default)]
    pub dependency_of: Option<String>,
    /// Version anterior guardada tras una actualizacion.
    #[serde(default)]
    pub previous: Option<PreviousVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousVersion {
    /// Archivo guardado en la carpeta de copias.
    pub file: String,
    pub version_id: String,
    pub version_number: String,
    pub sha1: String,
}

impl ModLock {
    pub fn find_project(&self, project_id: &str) -> Option<&LockedMod> {
        self.mods.iter().find(|m| m.project_id == project_id)
    }

    pub fn find_file(&self, file: &str) -> Option<&LockedMod> {
        self.mods.iter().find(|m| m.file == file)
    }

    /// Inserta o reemplaza por `project_id`.
    pub fn upsert(&mut self, entry: LockedMod) {
        match self.mods.iter_mut().find(|m| m.project_id == entry.project_id) {
            Some(existing) => *existing = entry,
            None => self.mods.push(entry),
        }
    }

    pub fn remove_file(&mut self, file: &str) -> Option<LockedMod> {
        let index = self.mods.iter().position(|m| m.file == file)?;
        Some(self.mods.remove(index))
    }
}

/// Lee el registro; ausente o ilegible -> vacio (los mods siguen funcionando, solo se pierde el origen).
pub async fn load_lock(path: &Path) -> ModLock {
    let Ok(data) = tokio::fs::read_to_string(path).await else {
        return ModLock { schema_version: SCHEMA_VERSION, mods: Vec::new() };
    };
    match serde_json::from_str::<ModLock>(&data) {
        Ok(mut lock) => {
            lock.schema_version = SCHEMA_VERSION;
            lock
        }
        Err(e) => {
            tracing::warn!(error = %e, "registro de mods ilegible; se ignora");
            ModLock { schema_version: SCHEMA_VERSION, mods: Vec::new() }
        }
    }
}

pub async fn save_lock(path: &Path, lock: &ModLock) -> Result<(), ModError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part = path.with_extension("json.part");
    let mut lock = lock.clone();
    lock.schema_version = SCHEMA_VERSION;
    tokio::fs::write(&part, serde_json::to_string_pretty(&lock)?).await?;
    tokio::fs::rename(&part, path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locked(project: &str, file: &str) -> LockedMod {
        LockedMod {
            file: file.into(),
            project_id: project.into(),
            version_id: "v1".into(),
            version_number: "1.0".into(),
            title: project.into(),
            sha1: "aa".into(),
            installed_at: Utc::now(),
            dependency_of: None,
            previous: None,
        }
    }

    #[test]
    fn upsert_replaces_by_project() {
        let mut lock = ModLock::default();
        lock.upsert(locked("p1", "a.jar"));
        lock.upsert(locked("p2", "b.jar"));
        let mut updated = locked("p1", "a2.jar");
        updated.version_id = "v2".into();
        lock.upsert(updated);

        assert_eq!(lock.mods.len(), 2);
        assert_eq!(lock.find_project("p1").unwrap().file, "a2.jar");
        assert!(lock.find_file("a.jar").is_none());
        assert_eq!(lock.remove_file("b.jar").unwrap().project_id, "p2");
    }

    #[tokio::test]
    async fn roundtrip_and_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".disorder").join("mods.json");

        assert!(load_lock(&path).await.mods.is_empty());

        let mut lock = ModLock::default();
        lock.upsert(locked("p1", "a.jar"));
        save_lock(&path, &lock).await.unwrap();
        assert_eq!(load_lock(&path).await.mods.len(), 1);

        std::fs::write(&path, "{ nope").unwrap();
        assert!(load_lock(&path).await.mods.is_empty());
    }
}
