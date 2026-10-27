//! Versiones de Minecraft instaladas fuera del launcher (la carpeta `.minecraft` del launcher
//! oficial: OptiFine, Forge o Fabric puestos a mano, perfiles de LiteLoader, etc.).
//!
//! Se muestran en la seccion «Personalizado» al crear una instancia. Al elegir una, se importa
//! a la carpeta del launcher como una version normal: el perfil se *fusiona* con la version en la
//! que se apoya (`inheritsFrom`), de modo que el resto del motor (instalacion, lanzamiento,
//! reparacion, skins) la trata como cualquier otra y no existe una segunda ruta de lanzamiento.
//!
//! - La version importada se llama `custom-<id original>`: nunca puede chocar con una version
//!   oficial ni con un loader instalado por el launcher.
//! - Las librerias que solo existen en el equipo (OptiFine, el `.jar` de Forge) se copian a
//!   `libraries/`; el resto se descarga como siempre.
//! - Nada se modifica ni se borra en la carpeta de origen: solo se lee.

use crate::{Launcher, LauncherError};
use instance_manager::{Instance, InstanceRepository, LoaderKind, NewInstance};
use minecraft_meta::{library_key, maven_path, VersionDescriptor};
use serde::Serialize;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Prefijo de las versiones importadas dentro del launcher.
pub const CUSTOM_PREFIX: &str = "custom-";
/// Maximo de perfiles encadenados (`inheritsFrom`) que se siguen.
const MAX_CHAIN: usize = 5;
/// Limite de `validate_version` de las instancias.
const MAX_ID_LEN: usize = 64;

/// Que es una version instalada fuera del launcher (solo para mostrarla con su nombre).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalKind {
    Optifine,
    Forge,
    Neoforge,
    Fabric,
    Quilt,
    Liteloader,
    Other,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalVersion {
    /// Nombre de la carpeta en `versions/` del origen (p. ej. `1.12.2-OptiFine_HD_U_E3`).
    pub id: String,
    /// Version de Minecraft en la que se apoya, si lo declara.
    pub base_version: Option<String>,
    pub kind: ExternalKind,
    pub release_time: Option<String>,
    /// Ya se importo antes al launcher.
    pub imported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalScan {
    /// Carpeta que se reviso.
    pub dir: String,
    /// `false` si la carpeta no existe o no tiene `versions/`.
    pub found: bool,
    pub versions: Vec<ExternalVersion>,
}

/// Carpeta `.minecraft` habitual de este sistema.
pub fn default_minecraft_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(".minecraft"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support/minecraft"))
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".minecraft"))
    }
}

fn version_json_path(root: &Path, id: &str) -> PathBuf {
    root.join("versions").join(id).join(format!("{id}.json"))
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Mismas reglas que `validate_version` de las instancias, porque el id es un nombre de carpeta.
fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN - CUSTOM_PREFIX.len()
        && !id.starts_with('.')
        && !id.contains("..")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ' '))
}

fn library_names(json: &Value) -> Vec<String> {
    json["libraries"]
        .as_array()
        .map(|libs| libs.iter().filter_map(|l| l["name"].as_str()).map(str::to_lowercase).collect())
        .unwrap_or_default()
}

fn kind_of(id: &str, json: &Value) -> ExternalKind {
    let id = id.to_lowercase();
    let libs = library_names(json);
    let has = |prefix: &str| libs.iter().any(|n| n.starts_with(prefix));
    if id.contains("optifine") || has("optifine:") {
        ExternalKind::Optifine
    } else if id.contains("neoforge") || has("net.neoforged") {
        ExternalKind::Neoforge
    } else if id.contains("forge") || has("net.minecraftforge") {
        ExternalKind::Forge
    } else if id.starts_with("fabric-loader") || has("net.fabricmc:fabric-loader") {
        ExternalKind::Fabric
    } else if id.starts_with("quilt-loader") || has("org.quiltmc:quilt-loader") {
        ExternalKind::Quilt
    } else if id.contains("liteloader") || has("com.mumfrey:liteloader") {
        ExternalKind::Liteloader
    } else {
        ExternalKind::Other
    }
}

/// ¿Es un perfil «hecho a mano» (con mods o loader) y no una version normal de Mojang?
/// Las versiones normales ya estan en la lista oficial.
fn is_custom(json: &Value) -> bool {
    if json["inheritsFrom"].is_string() {
        return true;
    }
    let libs = library_names(json);
    ["optifine:", "net.minecraftforge", "net.neoforged", "net.fabricmc", "org.quiltmc", "com.mumfrey"]
        .iter()
        .any(|p| libs.iter().any(|n| n.starts_with(p)))
}

/// Versiones personalizadas de `<root>/versions/`, de la mas reciente a la mas antigua. Bloqueante.
pub fn scan_external(root: &Path, imported: &HashSet<String>) -> Vec<ExternalVersion> {
    let Ok(dir) = std::fs::read_dir(root.join("versions")) else { return Vec::new() };
    let mut found: Vec<ExternalVersion> = dir
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().to_string();
            if !safe_id(&id) {
                return None;
            }
            let json = read_json(&version_json_path(root, &id))?;
            if !is_custom(&json) {
                return None;
            }
            Some(ExternalVersion {
                kind: kind_of(&id, &json),
                base_version: json["inheritsFrom"].as_str().map(str::to_string),
                release_time: json["releaseTime"].as_str().or_else(|| json["time"].as_str()).map(str::to_string),
                imported: imported.contains(&id),
                id,
            })
        })
        .collect();
    found.sort_by(|a, b| b.release_time.cmp(&a.release_time).then_with(|| a.id.cmp(&b.id)));
    found
}

/// Fusiona un perfil (`child`) sobre la version en la que se apoya (`parent`), como hace el launcher
/// oficial: el hijo manda en los campos simples, sus librerias van primero (y sustituyen a las del
/// padre con la misma coordenada) y los argumentos se concatenan.
pub fn merge_profiles(parent: &Value, child: &Value) -> Value {
    let mut out = parent.as_object().cloned().unwrap_or_default();
    let Some(child) = child.as_object() else { return Value::Object(out) };
    for (key, value) in child {
        match key.as_str() {
            "libraries" => {
                let own: Vec<Value> = value.as_array().cloned().unwrap_or_default();
                let keys: HashSet<String> =
                    own.iter().filter_map(|l| l["name"].as_str()).map(library_key).collect();
                let mut libs = own;
                libs.extend(
                    parent["libraries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|l| !l["name"].as_str().is_some_and(|n| keys.contains(&library_key(n))))
                        .cloned(),
                );
                out.insert("libraries".into(), Value::Array(libs));
            }
            "arguments" => {
                let mut merged = parent["arguments"].as_object().cloned().unwrap_or_default();
                for side in ["game", "jvm"] {
                    let mut list: Vec<Value> = merged.get(side).and_then(Value::as_array).cloned().unwrap_or_default();
                    list.extend(value[side].as_array().cloned().unwrap_or_default());
                    merged.insert(side.into(), Value::Array(list));
                }
                out.insert("arguments".into(), Value::Object(merged));
            }
            // El formato antiguo de argumentos sustituye al moderno si el hijo lo trae.
            "minecraftArguments" => {
                out.remove("arguments");
                out.insert(key.clone(), value.clone());
            }
            _ => {
                out.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(out)
}

fn file_sha1(path: &Path) -> std::io::Result<(String, u64)> {
    let bytes = std::fs::read(path)?;
    Ok((format!("{:x}", Sha1::digest(&bytes)), bytes.len() as u64))
}

/// Copia una carpeta sin seguir enlaces simbolicos. Los archivos que ya existen en destino no se pisan.
fn copy_tree(from: &Path, to: &Path, skip_names: &[&str]) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if skip_names.iter().any(|s| name.to_string_lossy() == *s) {
            continue;
        }
        let kind = entry.file_type()?;
        let dest = to.join(&name);
        if kind.is_symlink() {
            continue;
        } else if kind.is_dir() {
            copy_tree(&entry.path(), &dest, skip_names)?;
        } else if kind.is_file() && !dest.exists() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// Mundos, mods, ajustes y packs de la carpeta de origen. El pack de skin del launcher no viaja.
fn copy_user_data(root: &Path, game_dir: &Path) -> std::io::Result<()> {
    if root == game_dir {
        return Ok(());
    }
    for dir in ["saves", "mods", "config", "resourcepacks", "shaderpacks", "texturepacks"] {
        let from = root.join(dir);
        if from.is_dir() {
            copy_tree(&from, &game_dir.join(dir), &["DisorderLauncher-skin.zip"])?;
        }
    }
    for file in ["options.txt", "servers.dat"] {
        let from = root.join(file);
        let to = game_dir.join(file);
        if from.is_file() && !to.exists() {
            std::fs::copy(from, to)?;
        }
    }
    Ok(())
}

/// Resultado de preparar una version: que se debe escribir y copiar. Todo lo que toca disco ocurre
/// en `write_imported`, para poder probar la fusion sin red.
struct Prepared {
    descriptor: Value,
    /// Jar del juego a copiar (si hay uno local).
    jar_source: Option<PathBuf>,
    /// Librerias locales: (origen, destino relativo a `libraries/`).
    libraries: Vec<(PathBuf, String)>,
    missing: Vec<String>,
}

/// Normaliza las librerias sin datos de descarga y localiza sus archivos en el origen.
fn collect_local_libraries(merged: &mut Value, root: &Path, launcher_libraries: &Path) -> (Vec<(PathBuf, String)>, Vec<String>) {
    let mut copies = Vec::new();
    let mut missing = Vec::new();
    let Some(libs) = merged["libraries"].as_array_mut() else { return (copies, missing) };
    for lib in libs {
        let Some(name) = lib["name"].as_str().map(str::to_string) else { continue };
        let artifact_url = lib["downloads"]["artifact"]["url"].as_str().map(str::to_string);
        let no_download_info = lib["downloads"].is_null() && lib["url"].is_null() && lib["natives"].is_null();
        let empty_url = artifact_url.as_deref().is_some_and(str::is_empty);
        if !no_download_info && !empty_url {
            continue;
        }
        let relative = lib["downloads"]["artifact"]["path"]
            .as_str()
            .map(str::to_string)
            .or_else(|| maven_path(&name));
        let Some(relative) = relative else { continue };
        if relative.contains("..") || relative.starts_with('/') || relative.contains('\\') {
            missing.push(name);
            continue;
        }
        let destination = launcher_libraries.join(&relative);
        let source = root.join("libraries").join(&relative);
        if destination.is_file() {
            // ya esta en el launcher
        } else if source.is_file() {
            copies.push((source, relative.clone()));
        } else {
            missing.push(name);
        }
        if empty_url {
            // El launcher la buscara por su ruta Maven, sin intentar descargarla.
            if let Some(object) = lib.as_object_mut() {
                object.remove("downloads");
            }
        }
    }
    (copies, missing)
}

impl Launcher {
    /// Version personalizada que hay en `root` (o en la carpeta habitual), para el apartado «Personalizado».
    pub async fn external_versions_scan(&self, dir: Option<&Path>) -> Result<ExternalScan, LauncherError> {
        let root = match dir {
            Some(dir) => dir.to_path_buf(),
            None => match default_minecraft_dir() {
                Some(dir) => dir,
                None => return Ok(ExternalScan { dir: String::new(), found: false, versions: Vec::new() }),
            },
        };
        let imported = self.imported_custom_ids();
        let scan_root = root.clone();
        let versions = tokio::task::spawn_blocking(move || scan_external(&scan_root, &imported))
            .await
            .map_err(|_| LauncherError::Storage)?;
        let found = root.join("versions").is_dir();
        Ok(ExternalScan { dir: root.display().to_string(), found, versions })
    }

    /// Ids originales de lo que ya se importo.
    fn imported_custom_ids(&self) -> HashSet<String> {
        let mut ids = HashSet::new();
        if let Ok(dir) = std::fs::read_dir(self.paths().minecraft_dir().join("versions")) {
            for entry in dir.flatten() {
                if let Some(original) = entry.file_name().to_string_lossy().strip_prefix(CUSTOM_PREFIX) {
                    ids.insert(original.to_string());
                }
            }
        }
        ids
    }

    /// Crea una instancia con una version personalizada de `root`. Con `copy_user_data` se copian
    /// tambien los mundos, mods, ajustes y packs de esa carpeta (nada se mueve ni se borra en el origen).
    pub async fn instances_create_custom(
        &self,
        name: &str,
        root: &Path,
        version_id: &str,
        copy_data: bool,
    ) -> Result<Instance, LauncherError> {
        let profile_id = self.accounts_default().await?.ok_or(LauncherError::NoDefaultProfile)?;
        let launcher_id = self.import_external_version(root, version_id).await?;

        let instance = self
            .instances
            .create(NewInstance {
                name: name.to_string(),
                minecraft_version: launcher_id,
                loader: LoaderKind::Vanilla,
                loader_version: None,
                profile_id,
            })
            .await?;

        if copy_data {
            // La instancia queda reservada mientras se copia: no se puede lanzar ni editar.
            self.running_guard().insert(instance.id.clone(), None);
            let (from, to) = (root.to_path_buf(), instance.game_dir.clone());
            let copied = tokio::task::spawn_blocking(move || copy_user_data(&from, &to)).await;
            self.running_guard().remove(&instance.id);
            if !matches!(copied, Ok(Ok(()))) {
                if let Err(e) = self.instances.delete(&instance.id).await {
                    tracing::warn!(error = %e, "no se pudo limpiar la instancia tras una copia fallida");
                }
                return Err(LauncherError::InvalidInput(
                    "No se pudieron copiar tus mundos y mods. Revisa que haya espacio en el disco e inténtalo de nuevo.".into(),
                ));
            }
        }
        Ok(instance)
    }

    /// Importa `versions/<id>` de `root` y devuelve el id con el que queda en el launcher.
    pub(crate) async fn import_external_version(&self, root: &Path, id: &str) -> Result<String, LauncherError> {
        let invalid = |m: &str| LauncherError::InvalidInput(m.to_string());
        if !safe_id(id) {
            return Err(invalid("Esa version tiene un nombre que el launcher no puede usar."));
        }
        let launcher_id = format!("{CUSTOM_PREFIX}{id}");
        let data_dir = self.paths().minecraft_dir();

        let (chain, mut merged) = self.load_chain(root, id).await?;
        let root_id = chain.last().cloned().unwrap_or_else(|| id.to_string());
        let root_release = merged["__rootRelease"].take();
        if let Some(object) = merged.as_object_mut() {
            object.remove("__rootRelease");
        }

        let prepared = self.prepare(root, &launcher_id, id, &root_id, &chain, merged, root_release, &data_dir)?;
        if !prepared.missing.is_empty() {
            tracing::warn!(missing = ?prepared.missing, "faltan librerias de la version personalizada");
            return Err(LauncherError::InvalidInput(format!(
                "A esa version le faltan archivos en tu carpeta de Minecraft ({}). Ábrela una vez con el launcher oficial para que se complete y vuelve a intentarlo.",
                prepared.missing.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            )));
        }
        self.write_imported(&launcher_id, prepared, &data_dir).await?;
        Ok(launcher_id)
    }

    /// Lee el perfil y toda su cadena de padres. Devuelve los ids (del perfil a la raiz) y el JSON fusionado.
    async fn load_chain(&self, root: &Path, id: &str) -> Result<(Vec<String>, Value), LauncherError> {
        let first = read_json(&version_json_path(root, id))
            .ok_or_else(|| LauncherError::InvalidInput("No se pudo leer esa version en la carpeta elegida.".into()))?;
        let mut chain = vec![id.to_string()];
        let mut jsons = vec![first];
        while let Some(parent) = jsons.last().and_then(|j| j["inheritsFrom"].as_str()).map(str::to_string) {
            if chain.len() >= MAX_CHAIN || chain.contains(&parent) || !safe_id(&parent) {
                return Err(LauncherError::InvalidInput("Esa version depende de otras de forma no valida.".into()));
            }
            let json = match read_json(&version_json_path(root, &parent)) {
                Some(json) => json,
                // No esta en la carpeta de origen: es una version oficial, se descarga de Mojang.
                None => serde_json::to_value(self.resolve_descriptor(&parent).await?).map_err(|_| LauncherError::Storage)?,
            };
            chain.push(parent);
            jsons.push(json);
        }
        let mut merged = jsons.pop().unwrap_or(Value::Null);
        let root_release = merged["releaseTime"].clone();
        for child in jsons.iter().rev() {
            merged = merge_profiles(&merged, child);
        }
        merged["__rootRelease"] = root_release;
        Ok((chain, merged))
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &self,
        root: &Path,
        launcher_id: &str,
        original_id: &str,
        root_id: &str,
        chain: &[String],
        mut merged: Value,
        root_release: Value,
        data_dir: &Path,
    ) -> Result<Prepared, LauncherError> {
        let existing = version_json_path(data_dir, launcher_id);
        if let Some(old) = read_json(&existing) {
            if old["disorderCustom"] != json!(true) {
                return Err(LauncherError::InvalidInput("Ya existe una version del launcher con ese nombre.".into()));
            }
        }

        // Jar del juego: el propio, el que nombra `jar`, el de algun ancestro o uno ya instalado en el launcher.
        let mut candidates: Vec<PathBuf> = vec![root.join("versions").join(original_id).join(format!("{original_id}.jar"))];
        if let Some(jar) = merged["jar"].as_str().filter(|j| safe_id(j)) {
            candidates.push(root.join("versions").join(jar).join(format!("{jar}.jar")));
        }
        for ancestor in chain.iter().skip(1) {
            candidates.push(root.join("versions").join(ancestor).join(format!("{ancestor}.jar")));
            candidates.push(data_dir.join("versions").join(ancestor).join(format!("{ancestor}.jar")));
        }
        let jar_source = candidates.into_iter().find(|p| p.is_file());

        // `downloads.client` debe describir el jar que de verdad se va a usar (si no, reparar lo pisaria).
        match &jar_source {
            Some(jar) => {
                let (sha1, size) = file_sha1(jar).map_err(|_| LauncherError::Storage)?;
                if merged["downloads"]["client"]["sha1"].as_str() != Some(&sha1) {
                    let url = merged["downloads"]["client"]["url"].as_str().unwrap_or_default().to_string();
                    merged["downloads"]["client"] = json!({ "sha1": sha1, "size": size, "url": url });
                }
            }
            None if merged["downloads"]["client"].is_null() => {
                return Err(LauncherError::InvalidInput(
                    "No se encontro el archivo del juego de esa version en tu carpeta de Minecraft.".into(),
                ));
            }
            None => {} // se descargara de Mojang al jugar
        }

        let (libraries, missing) = collect_local_libraries(&mut merged, root, &data_dir.join("libraries"));

        if let Some(obj) = merged.as_object_mut() {
            obj.remove("jar");
            obj.insert("id".into(), json!(launcher_id));
            obj.insert("inheritsFrom".into(), json!(root_id));
            obj.insert("disorderCustom".into(), json!(true));
            obj.insert("disorderOriginalId".into(), json!(original_id));
            if root_release.is_string() {
                obj.insert("releaseTime".into(), root_release);
            }
        }
        serde_json::from_value::<VersionDescriptor>(merged.clone()).map_err(|e| {
            tracing::warn!(error = %e, "la version personalizada no tiene el formato esperado");
            LauncherError::InvalidInput("Esa version no tiene un formato que el launcher entienda.".into())
        })?;
        Ok(Prepared { descriptor: merged, jar_source, libraries, missing })
    }

    async fn write_imported(&self, launcher_id: &str, prepared: Prepared, data_dir: &Path) -> Result<(), LauncherError> {
        let dir = data_dir.join("versions").join(launcher_id);
        let libraries_dir = data_dir.join("libraries");
        let json_path = dir.join(format!("{launcher_id}.json"));
        let jar_dest = dir.join(format!("{launcher_id}.jar"));
        let body = serde_json::to_vec_pretty(&prepared.descriptor).map_err(|_| LauncherError::Storage)?;
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            std::fs::create_dir_all(&dir)?;
            for (source, relative) in &prepared.libraries {
                let dest = libraries_dir.join(relative);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(source, &dest)?;
            }
            if let Some(jar) = &prepared.jar_source {
                std::fs::copy(jar, &jar_dest)?;
            }
            fs_safe::write_atomic_blocking(&json_path, body)
        })
        .await
        .map_err(|_| LauncherError::Storage)?
        .map_err(|e| {
            tracing::error!(error = %e, "no se pudo importar la version personalizada");
            LauncherError::from_io(&e)
        })
    }

    /// Version de Minecraft en la que se apoya una instancia: la propia, o la base de una version
    /// personalizada. Es la que entienden Modrinth y los modpacks.
    pub(crate) async fn base_minecraft_version(&self, instance: &Instance) -> String {
        let path = self.paths().version_descriptor(&instance.minecraft_version);
        let base = tokio::fs::read(&path)
            .await
            .ok()
            .and_then(|raw| serde_json::from_slice::<Value>(&raw).ok())
            .filter(|v| v["disorderCustom"] == json!(true))
            .and_then(|v| v["inheritsFrom"].as_str().map(str::to_string));
        base.unwrap_or_else(|| instance.minecraft_version.clone())
    }
}

/// Nombre para mostrar de una version del launcher: sin el prefijo de las importadas.
pub fn display_version(id: &str) -> &str {
    id.strip_prefix(CUSTOM_PREFIX).unwrap_or(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str, body: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    const VANILLA: &str = r#"{
      "id": "1.12.2", "type": "release", "releaseTime": "2017-09-18T08:39:46+00:00",
      "mainClass": "net.minecraft.client.main.Main",
      "minecraftArguments": "--username ${auth_player_name} --version ${version_name}",
      "assets": "1.12", "assetIndex": {"id":"1.12","sha1":"aa","size":1,"totalSize":1,"url":"https://x/i.json"},
      "downloads": {"client": {"sha1": "ROOT", "size": 3, "url": "https://x/client.jar"}},
      "javaVersion": {"component": "jre-legacy", "majorVersion": 8},
      "libraries": [
        {"name": "org.ow2.asm:asm:5.0.3", "downloads": {"artifact": {"path": "org/ow2/asm/asm/5.0.3/asm-5.0.3.jar", "sha1": "a", "size": 1, "url": "https://x/asm.jar"}}},
        {"name": "net.minecraft:launchwrapper:1.12", "downloads": {"artifact": {"path": "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar", "sha1": "b", "size": 1, "url": "https://x/lw.jar"}}}
      ]
    }"#;

    const OPTIFINE: &str = r#"{
      "id": "1.12.2-OptiFine_HD_U_E3", "inheritsFrom": "1.12.2", "type": "release", "time": "2018-01-01T00:00:00+00:00",
      "releaseTime": "2018-01-01T00:00:00+00:00",
      "mainClass": "net.minecraft.launchwrapper.Launch",
      "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --tweakClass optifine.OptiFineTweaker",
      "libraries": [
        {"name": "optifine:OptiFine:1.12.2_HD_U_E3"},
        {"name": "net.minecraft:launchwrapper:1.12-of"},
        {"name": "net.minecraft:launchwrapper:1.12", "downloads": {"artifact": {"path": "net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar", "sha1": "z", "size": 2, "url": "https://x/lw2.jar"}}}
      ]
    }"#;

    #[test]
    fn profiles_are_merged_like_the_official_launcher_does() {
        let merged = merge_profiles(&serde_json::from_str(VANILLA).unwrap(), &serde_json::from_str(OPTIFINE).unwrap());
        assert_eq!(merged["mainClass"], "net.minecraft.launchwrapper.Launch");
        assert!(merged["minecraftArguments"].as_str().unwrap().contains("optifine.OptiFineTweaker"));
        assert_eq!(merged["assets"], "1.12", "lo que el perfil no dice se hereda");
        let names: Vec<_> = merged["libraries"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
        // Las del perfil primero; la del padre con la misma coordenada se descarta.
        assert_eq!(names, ["optifine:OptiFine:1.12.2_HD_U_E3", "net.minecraft:launchwrapper:1.12-of", "net.minecraft:launchwrapper:1.12", "org.ow2.asm:asm:5.0.3"]);
    }

    #[test]
    fn modern_arguments_are_concatenated() {
        let parent = json!({"arguments": {"game": ["--a"], "jvm": ["-x"]}});
        let child = json!({"arguments": {"game": ["--b"]}});
        let merged = merge_profiles(&parent, &child);
        assert_eq!(merged["arguments"]["game"], json!(["--a", "--b"]));
        assert_eq!(merged["arguments"]["jvm"], json!(["-x"]));
    }

    #[test]
    fn the_scan_lists_only_custom_versions_with_their_kind() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "versions/1.12.2/1.12.2.json", VANILLA);
        write(dir.path(), "versions/1.12.2-OptiFine_HD_U_E3/1.12.2-OptiFine_HD_U_E3.json", OPTIFINE);
        write(
            dir.path(),
            "versions/fabric-loader-0.15.0-1.20.1/fabric-loader-0.15.0-1.20.1.json",
            r#"{"id":"fabric-loader-0.15.0-1.20.1","inheritsFrom":"1.20.1","releaseTime":"2023-10-01T00:00:00+00:00","libraries":[{"name":"net.fabricmc:fabric-loader:0.15.0"}]}"#,
        );
        write(dir.path(), "versions/roto/roto.json", "{ no es json");
        let imported = HashSet::from(["fabric-loader-0.15.0-1.20.1".to_string()]);
        let found = scan_external(dir.path(), &imported);
        let summary: Vec<_> = found.iter().map(|v| (v.id.as_str(), v.kind, v.imported)).collect();
        assert_eq!(
            summary,
            [("fabric-loader-0.15.0-1.20.1", ExternalKind::Fabric, true), ("1.12.2-OptiFine_HD_U_E3", ExternalKind::Optifine, false)]
        );
        assert_eq!(found[1].base_version.as_deref(), Some("1.12.2"));
        assert!(scan_external(&dir.path().join("nada"), &HashSet::new()).is_empty());
    }

    #[test]
    fn local_libraries_are_found_and_unreachable_ones_reported() {
        let source = tempfile::tempdir().unwrap();
        let launcher = tempfile::tempdir().unwrap();
        write(source.path(), "libraries/optifine/OptiFine/1.12.2_HD_U_E3/OptiFine-1.12.2_HD_U_E3.jar", "of");
        write(source.path(), "libraries/net/minecraftforge/forge/1.12.2-14.23/forge-1.12.2-14.23.jar", "forge");
        let mut merged = json!({"libraries": [
            {"name": "optifine:OptiFine:1.12.2_HD_U_E3"},
            {"name": "net.minecraftforge:forge:1.12.2-14.23", "downloads": {"artifact": {"path": "net/minecraftforge/forge/1.12.2-14.23/forge-1.12.2-14.23.jar", "sha1": "", "size": 0, "url": ""}}},
            {"name": "optifine:launchwrapper-of:2.2"},
            {"name": "org.ow2.asm:asm:5.0.3", "downloads": {"artifact": {"path": "a", "sha1": "a", "size": 1, "url": "https://x"}}},
            {"name": "../../etc:passwd:1"}
        ]});
        let (copies, missing) = collect_local_libraries(&mut merged, source.path(), &launcher.path().join("libraries"));
        let copied: Vec<_> = copies.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(copied, ["optifine/OptiFine/1.12.2_HD_U_E3/OptiFine-1.12.2_HD_U_E3.jar", "net/minecraftforge/forge/1.12.2-14.23/forge-1.12.2-14.23.jar"]);
        assert_eq!(missing, ["optifine:launchwrapper-of:2.2"], "una libreria que no existe se avisa; una ruta insegura se ignora");
        assert!(merged["libraries"][1]["downloads"].is_null(), "una URL vacia deja de intentar descargarse");
        assert!(merged["libraries"][3]["downloads"].is_object());
    }

    #[test]
    fn ids_must_be_safe_folder_names() {
        for ok in ["1.12.2-OptiFine_HD_U_E3", "fabric-loader-0.15.0-1.20.1", "1.7.10-Forge10.13.4.1614-1.7.10"] {
            assert!(safe_id(ok), "{ok}");
        }
        for bad in ["", "..", "a/b", "a\\b", ".oculto", &"x".repeat(80)] {
            assert!(!safe_id(bad), "{bad}");
        }
        assert_eq!(display_version("custom-1.12.2-OptiFine"), "1.12.2-OptiFine");
        assert_eq!(display_version("1.12.2"), "1.12.2");
    }

    #[test]
    fn user_data_is_copied_without_the_skin_pack_and_without_overwriting() {
        let source = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        write(source.path(), "saves/Mundo/level.dat", "nuevo");
        write(source.path(), "mods/a.jar", "mod");
        write(source.path(), "resourcepacks/DisorderLauncher-skin.zip", "skin");
        write(source.path(), "resourcepacks/otro.zip", "pack");
        write(source.path(), "options.txt", "fov:1");
        write(game.path(), "options.txt", "fov:0");
        copy_user_data(source.path(), game.path()).unwrap();
        assert_eq!(std::fs::read_to_string(game.path().join("saves/Mundo/level.dat")).unwrap(), "nuevo");
        assert!(game.path().join("mods/a.jar").is_file());
        assert!(game.path().join("resourcepacks/otro.zip").is_file());
        assert!(!game.path().join("resourcepacks/DisorderLauncher-skin.zip").exists());
        assert_eq!(std::fs::read_to_string(game.path().join("options.txt")).unwrap(), "fov:0", "no se pisa lo que ya hay");
    }
}
