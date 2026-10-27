use crate::ArtifactError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Ruta del indice relativa a la carpeta de juego de la instancia.
pub const INDEX_FILE: &str = ".disorder/artifacts.json";
const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ArtifactSource {
    Local,
    #[serde(rename_all = "camelCase")]
    Modrinth { project_id: String, version_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactEntry {
    pub sha512: String,
    pub size: u64,
    /// Fecha de modificacion al adoptarlo: si no cambia, no hace falta volver a calcular el hash.
    pub mtime_ns: u64,
    pub source: ArtifactSource,
}

/// Que artefactos usa una instancia. Clave: ruta relativa a la carpeta de juego, con `/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactIndex {
    pub schema_version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, ArtifactEntry>,
}

impl Default for ArtifactIndex {
    fn default() -> Self {
        Self { schema_version: SCHEMA, entries: BTreeMap::new() }
    }
}

impl ArtifactIndex {
    /// Ausente o ilegible -> vacio: se reconstruye en la siguiente adopcion, sin perder archivos.
    pub fn load(game_dir: &Path) -> Self {
        let Ok(bytes) = fs::read(game_dir.join(INDEX_FILE)) else {
            return Self::default();
        };
        match serde_json::from_slice::<Self>(&bytes) {
            Ok(index) if index.schema_version <= SCHEMA => index,
            _ => Self::default(),
        }
    }

    /// `true` si el archivo existe pero no se puede interpretar (la limpieza no debe fiarse de el).
    pub fn is_unreadable(game_dir: &Path) -> bool {
        match fs::read(game_dir.join(INDEX_FILE)) {
            Ok(bytes) => serde_json::from_slice::<Self>(&bytes).map(|i| i.schema_version > SCHEMA).unwrap_or(true),
            Err(e) => e.kind() != std::io::ErrorKind::NotFound,
        }
    }

    pub fn save(&self, game_dir: &Path) -> Result<(), ArtifactError> {
        let path = game_dir.join(INDEX_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs_safe::write_atomic_blocking(&path, serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?)?;
        Ok(())
    }

    /// Anota un archivo ya presente en la instancia (p. ej. recien materializado desde el almacen),
    /// con su origen conocido. Lee tamano y fecha del propio archivo.
    pub fn record_file(game_dir: &Path, rel_path: &str, sha512: &str, source: ArtifactSource) -> Result<(), ArtifactError> {
        let meta = fs::metadata(game_dir.join(rel_path))?;
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let mut index = Self::load(game_dir);
        index.entries.insert(
            rel_path.to_string(),
            ArtifactEntry { sha512: sha512.to_string(), size: meta.len(), mtime_ns, source },
        );
        index.save(game_dir)
    }

    pub fn set_source(&mut self, rel_path: &str, source: ArtifactSource) {
        if let Some(entry) = self.entries.get_mut(rel_path) {
            entry.source = source;
        }
    }
}
