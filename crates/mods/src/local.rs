use crate::{load_lock, read_mod_meta, save_lock, LockedMod, ModError, ModMeta};
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

pub const DISABLED_SUFFIX: &str = ".disabled";
const MAX_FILE_NAME: usize = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    /// Nombre real en disco (puede terminar en `.disabled`).
    pub file_name: String,
    pub enabled: bool,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub size: u64,
    #[serde(skip)]
    pub meta: Option<ModMeta>,
    /// De donde se instalo, si fue desde Modrinth.
    pub source: Option<ModSource>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSource {
    pub project_id: String,
    pub version_id: String,
    pub version_number: String,
    pub has_previous_version: bool,
}

/// Un mod activo que necesita otro que no esta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingDependency {
    pub mod_name: String,
    /// Id del mod que falta tal como lo declara el mod que lo pide (p. ej. `fabric-api`).
    pub id: String,
    /// Nombre legible de lo que falta (p. ej. "Fabric API").
    pub requires: String,
}

/// Acceso a la carpeta de mods de una instancia: `<game_dir>/mods`.
pub struct ModManager {
    mods_dir: PathBuf,
    state_dir: PathBuf,
}

impl ModManager {
    pub fn new(game_dir: impl AsRef<Path>) -> Self {
        let game_dir = game_dir.as_ref();
        Self { mods_dir: game_dir.join("mods"), state_dir: game_dir.join(".disorder") }
    }

    pub fn mods_dir(&self) -> &Path {
        &self.mods_dir
    }

    /// Carpeta de juego de la instancia (la que contiene `mods/`).
    pub fn game_dir(&self) -> &Path {
        self.mods_dir.parent().unwrap_or(&self.mods_dir)
    }

    /// Archivo con el registro de origen de los mods instalados desde Modrinth.
    pub fn lock_file(&self) -> PathBuf {
        self.state_dir.join("mods.json")
    }

    pub(crate) fn lock_path(&self) -> PathBuf {
        self.lock_file()
    }

    pub(crate) fn backup_dir(&self) -> PathBuf {
        self.state_dir.join("backup")
    }

    pub async fn list(&self) -> Result<Vec<ModEntry>, ModError> {
        let mut dir = match fs::read_dir(&self.mods_dir).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let lock = load_lock(&self.lock_path()).await;

        let mut entries = Vec::new();
        while let Some(item) = dir.next_entry().await? {
            let file_name = item.file_name().to_string_lossy().into_owned();
            let Some((base, enabled)) = split_mod_file(&file_name) else { continue };
            if !item.file_type().await?.is_file() {
                continue;
            }
            let size = item.metadata().await?.len();

            let path = item.path();
            let meta = tokio::task::spawn_blocking(move || read_mod_meta(&path))
                .await
                .ok()
                .and_then(Result::ok);

            let source = lock.find_file(base).map(|l| ModSource {
                project_id: l.project_id.clone(),
                version_id: l.version_id.clone(),
                version_number: l.version_number.clone(),
                has_previous_version: l.previous.is_some(),
            });
            entries.push(ModEntry {
                name: meta.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| base.trim_end_matches(".jar").to_string()),
                version: meta.as_ref().map(|m| m.version.clone()).filter(|v| !v.is_empty()),
                description: meta.as_ref().and_then(|m| m.description.clone()),
                file_name,
                enabled,
                size,
                meta,
                source,
            });
        }
        entries.sort_by_key(|e| e.name.to_lowercase());
        Ok(entries)
    }

    pub async fn set_enabled(&self, file_name: &str, enabled: bool) -> Result<String, ModError> {
        let (base, currently_enabled) = split_mod_file(file_name).ok_or(ModError::InvalidFileName)?;
        validate_file_name(file_name)?;
        if currently_enabled == enabled {
            return Ok(file_name.to_string());
        }
        let target = if enabled { base.to_string() } else { format!("{base}{DISABLED_SUFFIX}") };
        let from = self.mods_dir.join(file_name);
        if !from.is_file() {
            return Err(ModError::NotFound);
        }
        fs_safe::rename(&from, self.mods_dir.join(&target)).await?;
        Ok(target)
    }

    pub async fn remove(&self, file_name: &str) -> Result<(), ModError> {
        let (base, _) = split_mod_file(file_name).ok_or(ModError::InvalidFileName)?;
        validate_file_name(file_name)?;
        let path = self.mods_dir.join(file_name);
        if !path.is_file() {
            return Err(ModError::NotFound);
        }
        fs_safe::remove_file(&path).await?;

        let mut lock = load_lock(&self.lock_path()).await;
        if let Some(entry) = lock.remove_file(base) {
            if let Some(previous) = entry.previous {
                let _ = fs::remove_file(self.backup_dir().join(previous.file)).await;
            }
            save_lock(&self.lock_path(), &lock).await?;
        }
        Ok(())
    }

    /// Copia un jar elegido por el usuario. Debe ser un mod valido (Fabric, Quilt, Forge o NeoForge).
    pub async fn add_local(&self, source: &Path) -> Result<ModEntry, ModError> {
        let file_name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or(ModError::InvalidFileName)?;
        validate_file_name(&file_name)?;
        if !file_name.ends_with(".jar") {
            return Err(ModError::InvalidFileName);
        }

        let check = source.to_path_buf();
        tokio::task::spawn_blocking(move || read_mod_meta(&check))
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))??;

        fs::create_dir_all(&self.mods_dir).await?;
        let dest = self.mods_dir.join(&file_name);
        let part = self.mods_dir.join(format!("{file_name}.part"));
        fs::copy(source, &part).await?;
        fs_safe::rename(&part, &dest).await?;

        self.list()
            .await?
            .into_iter()
            .find(|e| e.file_name == file_name)
            .ok_or(ModError::NotFound)
    }

    /// Anota los mods activos que necesitan otro mod que no esta instalado/activo.
    pub fn missing_dependencies(entries: &[ModEntry]) -> Vec<MissingDependency> {
        let provided: HashSet<&str> = entries
            .iter()
            .filter(|e| e.enabled)
            .filter_map(|e| e.meta.as_ref())
            .flat_map(|m| std::iter::once(m.id.as_str()).chain(m.provides.iter().map(String::as_str)))
            .collect();

        let mut missing = Vec::new();
        for entry in entries.iter().filter(|e| e.enabled) {
            let Some(meta) = &entry.meta else { continue };
            for required in &meta.requires {
                if !provided.contains(required.as_str()) {
                    missing.push(MissingDependency {
                        mod_name: entry.name.clone(),
                        id: required.clone(),
                        requires: friendly_dependency_name(required),
                    });
                }
            }
        }
        missing
    }

    pub(crate) async fn record(&self, entry: LockedMod) -> Result<(), ModError> {
        let mut lock = load_lock(&self.lock_path()).await;
        lock.upsert(entry);
        save_lock(&self.lock_path(), &lock).await
    }
}

/// Nombres legibles para dependencias frecuentes; el resto se muestra con su id.
pub fn friendly_dependency_name(id: &str) -> String {
    match id {
        "fabric-api" | "fabric" => "Fabric API".to_string(),
        "cloth-config" | "cloth-config2" => "Cloth Config".to_string(),
        "modmenu" => "Mod Menu".to_string(),
        other => other.to_string(),
    }
}

/// `(nombre_base.jar, activo)` para `x.jar` y `x.jar.disabled`; `None` para otros archivos.
pub(crate) fn split_mod_file(name: &str) -> Option<(&str, bool)> {
    if name.ends_with(".jar") {
        Some((name, true))
    } else {
        name.strip_suffix(DISABLED_SUFFIX).filter(|b| b.ends_with(".jar")).map(|b| (b, false))
    }
}

/// Solo nombres simples: sin rutas, sin `..`, sin caracteres de control ni `:` (flujos NTFS).
pub(crate) fn validate_file_name(name: &str) -> Result<(), ModError> {
    let bad = name.is_empty()
        || name.len() > MAX_FILE_NAME
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '\0'])
        || name.chars().any(char::is_control)
        || Path::new(name).file_name().map(|n| n.to_string_lossy() != name).unwrap_or(true);
    if bad || split_mod_file(name).is_none() && !name.ends_with(".part") {
        return Err(ModError::InvalidFileName);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    pub(crate) fn make_mod(dir: &Path, file: &str, meta_json: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(file);
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        writer.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(meta_json.as_bytes()).unwrap();
        writer.finish().unwrap();
        path
    }

    fn make_forge_mod(dir: &Path, file: &str, toml: &str, entry: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(file);
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        writer.start_file(entry, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(toml.as_bytes()).unwrap();
        writer.finish().unwrap();
        path
    }

    #[tokio::test]
    async fn lists_forge_and_neoforge_mods_with_their_dependencies() {
        let (dir, m) = game();
        let src = dir.path().join("src");
        let forge = "[[mods]]
modId = \"jei\"
version = \"1.0\"
displayName = \"JEI\"
[[dependencies.jei]]
modId = \"cloth\"
mandatory = true
";
        let neo = "[[mods]]
modId = \"cloth\"
displayName = \"Cloth\"
";
        let a = make_forge_mod(&src, "jei.jar", forge, "META-INF/mods.toml");
        let b = make_forge_mod(&src, "cloth.jar", neo, "META-INF/neoforge.mods.toml");
        m.add_local(&a).await.unwrap();
        let entries = m.list().await.unwrap();
        assert_eq!(entries[0].name, "JEI");
        assert_eq!(ModManager::missing_dependencies(&entries).len(), 1);
        m.add_local(&b).await.unwrap();
        assert!(ModManager::missing_dependencies(&m.list().await.unwrap()).is_empty());
    }

    fn game() -> (tempfile::TempDir, ModManager) {
        let dir = tempfile::tempdir().unwrap();
        let manager = ModManager::new(dir.path().join("game"));
        (dir, manager)
    }

    #[test]
    fn file_name_rules() {
        for ok in ["a.jar", "a b-1.0.jar", "a.jar.disabled"] {
            assert!(validate_file_name(ok).is_ok(), "{ok}");
        }
        for bad in ["", "../a.jar", "a/b.jar", "a\\b.jar", "a.jar:stream", "a.txt", "..", "a\nb.jar"] {
            assert!(validate_file_name(bad).is_err(), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn lists_mods_with_metadata_and_state() {
        let (_d, m) = game();
        make_mod(m.mods_dir(), "sodium.jar", r#"{"id":"sodium","name":"Sodium","version":"1.0"}"#);
        make_mod(m.mods_dir(), "lithium.jar.disabled", r#"{"id":"lithium","name":"Lithium","version":"2.0"}"#);
        std::fs::write(m.mods_dir().join("readme.txt"), "x").unwrap();
        std::fs::write(m.mods_dir().join("broken.jar"), "not a zip").unwrap();

        let list = m.list().await.unwrap();
        let names: Vec<_> = list.iter().map(|e| (e.name.as_str(), e.enabled)).collect();
        assert_eq!(names, [("broken", true), ("Lithium", false), ("Sodium", true)]);
        assert!(list[0].meta.is_none(), "un jar sin metadatos sigue listado");
    }

    #[tokio::test]
    async fn empty_or_missing_dir_lists_nothing() {
        let (_d, m) = game();
        assert!(m.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn enable_disable_roundtrip() {
        let (_d, m) = game();
        make_mod(m.mods_dir(), "a.jar", r#"{"id":"a"}"#);

        assert_eq!(m.set_enabled("a.jar", false).await.unwrap(), "a.jar.disabled");
        assert!(!m.mods_dir().join("a.jar").exists());
        assert_eq!(m.set_enabled("a.jar.disabled", false).await.unwrap(), "a.jar.disabled", "idempotente");
        assert_eq!(m.set_enabled("a.jar.disabled", true).await.unwrap(), "a.jar");
        assert!(m.mods_dir().join("a.jar").is_file());

        assert!(matches!(m.set_enabled("../a.jar", false).await, Err(ModError::InvalidFileName)));
        assert!(matches!(m.set_enabled("nope.jar", false).await, Err(ModError::NotFound)));
    }

    #[tokio::test]
    async fn remove_deletes_file() {
        let (_d, m) = game();
        make_mod(m.mods_dir(), "a.jar", r#"{"id":"a"}"#);
        m.remove("a.jar").await.unwrap();
        assert!(m.list().await.unwrap().is_empty());
        assert!(matches!(m.remove("a.jar").await, Err(ModError::NotFound)));
        assert!(matches!(m.remove("../x.jar").await, Err(ModError::InvalidFileName)));
    }

    #[tokio::test]
    async fn add_local_validates_the_jar() {
        let (d, m) = game();
        let good = make_mod(&d.path().join("downloads"), "cool.jar", r#"{"id":"cool","name":"Cool"}"#);
        let entry = m.add_local(&good).await.unwrap();
        assert_eq!(entry.name, "Cool");
        assert!(m.mods_dir().join("cool.jar").is_file());
        assert!(!m.mods_dir().join("cool.jar.part").exists());

        let fake = d.path().join("downloads").join("fake.jar");
        std::fs::write(&fake, "not a jar").unwrap();
        assert!(matches!(m.add_local(&fake).await, Err(ModError::NotAMod)));

        let txt = d.path().join("downloads").join("notes.txt");
        std::fs::write(&txt, "x").unwrap();
        assert!(matches!(m.add_local(&txt).await, Err(ModError::InvalidFileName)));
    }

    #[tokio::test]
    async fn reports_missing_dependencies_in_friendly_terms() {
        let (_d, m) = game();
        make_mod(m.mods_dir(), "a.jar", r#"{"id":"a","name":"Alpha","depends":{"fabric-api":"*","fabricloader":"*"}}"#);
        make_mod(m.mods_dir(), "b.jar", r#"{"id":"b","name":"Beta","depends":{"a":"*"}}"#);

        let missing = ModManager::missing_dependencies(&m.list().await.unwrap());
        assert_eq!(missing, vec![MissingDependency { mod_name: "Alpha".into(), id: "fabric-api".into(), requires: "Fabric API".into() }]);

        make_mod(m.mods_dir(), "fabric-api.jar", r#"{"id":"fabric-api","name":"Fabric API"}"#);
        assert!(ModManager::missing_dependencies(&m.list().await.unwrap()).is_empty());

        // Un mod desactivado no cubre la dependencia.
        m.set_enabled("fabric-api.jar", false).await.unwrap();
        assert_eq!(ModManager::missing_dependencies(&m.list().await.unwrap()).len(), 1);
    }
}
