use crate::InstanceError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_NAME_LEN: usize = 64;
/// Minecraft no arranca de forma fiable con menos heap que esto.
pub const MIN_MAX_MEMORY_MB: u32 = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    /// Referencia a la cuenta. Nunca tokens (ver spec 06 §3).
    pub profile_id: String,
    pub java: JavaSelection,
    pub memory: MemoryPolicy,
    /// Derivado de la ubicacion en disco; no se guarda en `instance.json`.
    pub game_dir: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum JavaSelection {
    Auto,
    Path { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum MemoryPolicy {
    Auto,
    #[serde(rename_all = "camelCase")]
    Fixed { min_mb: u32, max_mb: u32 },
}

#[derive(Debug, Clone)]
pub struct NewInstance {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub profile_id: String,
}

/// Cambios parciales; `None` deja el campo como esta.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceUpdate {
    pub name: Option<String>,
    pub minecraft_version: Option<String>,
    pub profile_id: Option<String>,
    pub java: Option<JavaSelection>,
    pub memory: Option<MemoryPolicy>,
}

pub(crate) fn validate_name(name: &str) -> Result<String, InstanceError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(InstanceError::Invalid("el nombre no puede estar vacio".into()));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(InstanceError::Invalid(format!(
            "el nombre no puede superar {MAX_NAME_LEN} caracteres"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(InstanceError::Invalid("el nombre contiene caracteres no permitidos".into()));
    }
    Ok(name.to_string())
}

pub(crate) fn validate_memory(memory: &MemoryPolicy) -> Result<(), InstanceError> {
    if let MemoryPolicy::Fixed { min_mb, max_mb } = memory {
        if *min_mb == 0 || min_mb > max_mb {
            return Err(InstanceError::Invalid(
                "la memoria minima debe ser mayor que 0 y no superar la maxima".into(),
            ));
        }
        if *max_mb < MIN_MAX_MEMORY_MB {
            return Err(InstanceError::Invalid(format!(
                "la memoria maxima debe ser de al menos {MIN_MAX_MEMORY_MB} MB"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_required(field: &str, value: &str) -> Result<(), InstanceError> {
    if value.trim().is_empty() {
        return Err(InstanceError::Invalid(format!("{field} es obligatorio")));
    }
    Ok(())
}
