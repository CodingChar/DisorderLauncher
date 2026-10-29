use crate::path::validate_entry_path;
use crate::DpackError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MANIFEST_NAME: &str = "dpack-manifest.json";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecraftInfo {
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderInfo {
    /// `vanilla`, `fabric`, `quilt`, `forge` o `neoforge`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// De donde vino el archivo. Solo informativo: el contenido siempre viaja dentro del paquete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EntrySource {
    Local,
    Modrinth { project_id: String, version_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// Ruta relativa a la carpeta de juego, con `/`.
    pub path: String,
    pub size: u64,
    pub sha512: String,
    pub source: EntrySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub pack: PackInfo,
    pub minecraft: MinecraftInfo,
    pub loader: LoaderInfo,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    /// Comprueba todo lo que el importador va a usar como ruta o identidad. Nada del paquete se
    /// fia hasta pasar por aqui.
    pub fn validate(&self, max_entries: usize, max_total_bytes: u64) -> Result<(), DpackError> {
        if self.format_version == 0 {
            return Err(DpackError::Manifest("version de formato 0".into()));
        }
        if self.format_version > FORMAT_VERSION {
            return Err(DpackError::UnsupportedVersion(self.format_version));
        }
        if self.minecraft.version.trim().is_empty() || self.minecraft.version.len() > 64 {
            return Err(DpackError::Manifest("falta la version de Minecraft".into()));
        }
        if self.entries.len() > max_entries {
            return Err(DpackError::TooManyEntries);
        }
        let mut seen = HashSet::new();
        let mut total = 0u64;
        for entry in &self.entries {
            validate_entry_path(&entry.path)?;
            if !artifact_store::is_valid_hash(&entry.sha512) {
                return Err(DpackError::Manifest(format!("hash no valido en {}", entry.path)));
            }
            // En Windows `A.jar` y `a.jar` son el mismo archivo.
            if !seen.insert(entry.path.to_lowercase()) {
                return Err(DpackError::Duplicate(entry.path.clone()));
            }
            total = total.checked_add(entry.size).ok_or(DpackError::TooLarge)?;
        }
        if total > max_total_bytes {
            return Err(DpackError::TooLarge);
        }
        Ok(())
    }
}
