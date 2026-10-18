use crate::ModError;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Limite del JSON de metadatos: un mod legitimo ocupa unos KB.
const MAX_METADATA_BYTES: u64 = 1024 * 1024;

/// Metadatos de un mod (Fabric/Quilt: `fabric.mod.json`; Forge/NeoForge: `mods.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// Ids de mods requeridos (sin `minecraft`, `java` ni el propio loader).
    pub requires: Vec<String>,
    /// Ids adicionales que este mod aporta (`provides`).
    pub provides: Vec<String>,
}

/// Nombre anterior, cuando solo se leian mods de Fabric.
pub type FabricModMeta = ModMeta;

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

pub fn parse_fabric_meta(json: &[u8]) -> Result<ModMeta, ModError> {
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
    Ok(ModMeta {
        name: raw.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| raw.id.clone()),
        id: raw.id,
        version: raw.version.unwrap_or_default(),
        description: raw.description.filter(|d| !d.trim().is_empty()),
        requires,
        provides: raw.provides,
    })
}

/// Dependencias de Forge/NeoForge que no son mods instalables.
const BUILTIN_TOML_DEPENDENCIES: &[&str] = &["minecraft", "forge", "neoforge", "java", "fml", "javafml", "lowcodefml"];

/// Interpreta `META-INF/mods.toml` (Forge) o `META-INF/neoforge.mods.toml` (NeoForge).
pub fn parse_forge_meta(toml_text: &str) -> Result<ModMeta, ModError> {
    let doc: toml::Table = toml_text.parse().map_err(|_| ModError::NotAMod)?;
    let first = doc
        .get("mods")
        .and_then(toml::Value::as_array)
        .and_then(|mods| mods.first())
        .and_then(toml::Value::as_table)
        .ok_or(ModError::NotAMod)?;
    let id = first.get("modId").and_then(toml::Value::as_str).unwrap_or_default().trim().to_string();
    if id.is_empty() {
        return Err(ModError::NotAMod);
    }
    let text = |key: &str| first.get(key).and_then(toml::Value::as_str).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
    // `${file.jarVersion}` y similares se rellenan al compilar el mod: si quedan sin resolver, no se muestran.
    let version = text("version").filter(|v| !v.contains("${")).unwrap_or_default();

    let mut requires: Vec<String> = Vec::new();
    if let Some(deps) = doc.get("dependencies").and_then(toml::Value::as_table).and_then(|d| d.get(&id)).and_then(toml::Value::as_array) {
        for dep in deps.iter().filter_map(toml::Value::as_table) {
            let dep_id = dep.get("modId").and_then(toml::Value::as_str).unwrap_or_default();
            // Forge usa `mandatory = true`; NeoForge usa `type = "required"`.
            let required = dep.get("mandatory").and_then(toml::Value::as_bool).unwrap_or(false)
                || dep.get("type").and_then(toml::Value::as_str).is_some_and(|t| t.eq_ignore_ascii_case("required"));
            if required && !dep_id.is_empty() && !BUILTIN_TOML_DEPENDENCIES.contains(&dep_id) {
                requires.push(dep_id.to_string());
            }
        }
    }
    requires.sort();
    requires.dedup();
    Ok(ModMeta {
        name: text("displayName").unwrap_or_else(|| id.clone()),
        id,
        version,
        description: text("description"),
        requires,
        provides: Vec::new(),
    })
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let entry = zip.by_name(name).ok()?;
    if entry.size() > MAX_METADATA_BYTES {
        return None;
    }
    let mut data = Vec::with_capacity(entry.size() as usize);
    entry.take(MAX_METADATA_BYTES).read_to_end(&mut data).ok()?;
    Some(data)
}

/// Lee los metadatos de un jar de mod (Fabric/Quilt, NeoForge o Forge). Bloqueante.
pub fn read_mod_meta(jar: &Path) -> Result<ModMeta, ModError> {
    let file = std::fs::File::open(jar)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| ModError::NotAMod)?;
    if let Some(data) = read_entry(&mut zip, "fabric.mod.json") {
        return parse_fabric_meta(&data);
    }
    for name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(data) = read_entry(&mut zip, name) {
            let text = String::from_utf8(data).map_err(|_| ModError::NotAMod)?;
            return parse_forge_meta(&text);
        }
    }
    Err(ModError::NotAMod)
}

/// Nombre anterior de `read_mod_meta`.
pub fn read_fabric_meta(jar: &Path) -> Result<ModMeta, ModError> {
    read_mod_meta(jar)
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

    #[test]
    fn parses_forge_toml_with_mandatory_dependencies() {
        let toml = r#"
modLoader = "javafml"
[[mods]]
modId = "jei"
version = "${file.jarVersion}"
displayName = "Just Enough Items"
description = "Recetas"
[[dependencies.jei]]
modId = "forge"
mandatory = true
[[dependencies.jei]]
modId = "cloth_config"
mandatory = true
[[dependencies.jei]]
modId = "optional_thing"
mandatory = false
"#;
        let meta = parse_forge_meta(toml).unwrap();
        assert_eq!(meta.id, "jei");
        assert_eq!(meta.name, "Just Enough Items");
        assert_eq!(meta.version, "");
        assert_eq!(meta.requires, vec!["cloth_config".to_string()]);
    }

    #[test]
    fn parses_neoforge_required_type() {
        let toml = r#"
[[mods]]
modId = "foo"
version = "1.2"
[[dependencies.foo]]
modId = "neoforge"
type = "required"
[[dependencies.foo]]
modId = "bar"
type = "required"
"#;
        let meta = parse_forge_meta(toml).unwrap();
        assert_eq!(meta.version, "1.2");
        assert_eq!(meta.requires, vec!["bar".to_string()]);
        assert!(parse_forge_meta("nope = [").is_err());
        assert!(parse_forge_meta("[[mods]]
modId = \"\"").is_err());
    }
}
