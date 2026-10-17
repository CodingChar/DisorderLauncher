use crate::ModError;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Limite del JSON de metadatos: un mod legitimo ocupa unos KB.
const MAX_METADATA_BYTES: u64 = 1024 * 1024;

/// Metadatos de un mod de Fabric (`fabric.mod.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricModMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// Ids de mods requeridos (sin `minecraft`, `java` ni el propio loader).
    pub requires: Vec<String>,
    /// Ids adicionales que este mod aporta (`provides`).
    pub provides: Vec<String>,
}

#[derive(Deserialize)]
struct RawFabricMeta {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    depends: HashMap<String, serde_json::Value>,
    #[serde(default)]
    provides: Vec<String>,
}

/// Dependencias que no son mods instalables.
const BUILTIN_DEPENDENCIES: &[&str] = &["minecraft", "java", "fabricloader", "fabric-loader"];

pub fn parse_fabric_meta(json: &[u8]) -> Result<FabricModMeta, ModError> {
    let raw: RawFabricMeta = serde_json::from_slice(json)?;
    if raw.id.trim().is_empty() {
        return Err(ModError::NotAMod);
    }
    let mut requires: Vec<String> = raw
        .depends
        .into_keys()
        .filter(|id| !BUILTIN_DEPENDENCIES.contains(&id.as_str()))
        .collect();
    requires.sort();
    Ok(FabricModMeta {
        name: raw.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| raw.id.clone()),
        id: raw.id,
        version: raw.version.unwrap_or_default(),
        description: raw.description.filter(|d| !d.trim().is_empty()),
        requires,
        provides: raw.provides,
    })
}

/// Lee `fabric.mod.json` de un jar. Bloqueante.
pub fn read_fabric_meta(jar: &Path) -> Result<FabricModMeta, ModError> {
    let file = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| ModError::NotAMod)?;
    let entry = zip.by_name("fabric.mod.json").map_err(|_| ModError::NotAMod)?;
    if entry.size() > MAX_METADATA_BYTES {
        return Err(ModError::NotAMod);
    }
    let mut data = Vec::with_capacity(entry.size() as usize);
    entry.take(MAX_METADATA_BYTES).read_to_end(&mut data)?;
    parse_fabric_meta(&data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_core_fields_and_filters_builtin_dependencies() {
        let json = br#"{
          "schemaVersion": 1, "id": "sodium", "version": "0.5.8", "name": "Sodium",
          "description": "Rendering engine",
          "depends": { "fabricloader": ">=0.15", "minecraft": "1.20.x", "java": ">=17", "fabric-api": "*" },
          "provides": ["rubidium"]
        }"#;
        let meta = parse_fabric_meta(json).unwrap();
        assert_eq!(meta.id, "sodium");
        assert_eq!(meta.name, "Sodium");
        assert_eq!(meta.requires, vec!["fabric-api".to_string()]);
        assert_eq!(meta.provides, vec!["rubidium".to_string()]);
    }

    #[test]
    fn name_falls_back_to_id() {
        let meta = parse_fabric_meta(br#"{"id":"foo"}"#).unwrap();
        assert_eq!(meta.name, "foo");
        assert!(meta.requires.is_empty());
    }

    #[test]
    fn empty_id_is_not_a_mod() {
        assert!(parse_fabric_meta(br#"{"id":" "}"#).is_err());
        assert!(parse_fabric_meta(b"nope").is_err());
    }
}
