use crate::{validate_relative_path, FileEnv, FileHashes, ModpackError, ModpackFile, ModpackIndex, INDEX_FILE};
use instance_manager::LoaderKind;
use mods::{load_lock, ModrinthClient};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Carpetas y archivos de la instancia que viajan dentro del modpack. Nunca se exportan
/// mundos, logs, capturas ni el estado interno del launcher.
const EXPORT_DIRS: &[&str] = &["config", "mods", "resourcepacks", "shaderpacks", "datapacks"];
const EXPORT_FILES: &[&str] = &["options.txt"];
const MAX_EXPORT_FILES: usize = 20_000;
/// Resource pack con la skin del perfil que genera el launcher al lanzar (`launcher-core/skin_pack.rs`):
/// es personal y se rehace sola, asi que no viaja en el paquete.
const GENERATED_SKIN_PACK: &str = "DisorderLauncher-skin.zip";

pub struct ExportInput<'a> {
    pub name: &'a str,
    pub version_id: &'a str,
    pub summary: Option<&'a str>,
    pub minecraft_version: &'a str,
    pub loader: Option<(LoaderKind, &'a str)>,
    pub game_dir: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    /// Mods referenciados por URL de Modrinth (con hash).
    pub remote_files: usize,
    /// Archivos incluidos dentro del paquete (`overrides/`).
    pub bundled_files: usize,
}

fn loader_dependency(kind: LoaderKind) -> Option<&'static str> {
    match kind {
        LoaderKind::Fabric => Some("fabric-loader"),
        LoaderKind::Quilt => Some("quilt-loader"),
        LoaderKind::Forge => Some("forge"),
        LoaderKind::NeoForge => Some("neoforge"),
        LoaderKind::Vanilla => None,
    }
}

/// Crea un `.mrpack`. Los mods instalados desde Modrinth se referencian por URL y hash;
/// si no se puede consultar la red, o el mod no viene de Modrinth, viaja en `overrides/`.
pub async fn export_mrpack(
    http: &reqwest::Client,
    input: &ExportInput<'_>,
    dest: &Path,
) -> Result<ExportReport, ModpackError> {
    let remote = remote_mod_files(http, input.game_dir).await;
    let remote_paths: HashSet<String> = remote.iter().map(|f| f.path.clone()).collect();

    let bundled = collect_bundled(input.game_dir, &remote_paths)?;

    let mut dependencies = BTreeMap::new();
    dependencies.insert("minecraft".to_string(), input.minecraft_version.to_string());
    if let Some((kind, version)) = input.loader {
        if let Some(key) = loader_dependency(kind) {
            dependencies.insert(key.to_string(), version.to_string());
        }
    }

    let index = ModpackIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: input.version_id.to_string(),
        name: input.name.to_string(),
        summary: input.summary.map(str::to_string),
        files: remote,
        dependencies,
    };

    let report = ExportReport { remote_files: index.files.len(), bundled_files: bundled.len() };
    let dest = dest.to_path_buf();
    let game_dir = input.game_dir.to_path_buf();
    tokio::task::spawn_blocking(move || write_package(&dest, &index, &game_dir, &bundled))
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))??;
    Ok(report)
}

/// Mods activos registrados desde Modrinth, con URL y hashes consultados a la API.
async fn remote_mod_files(http: &reqwest::Client, game_dir: &Path) -> Vec<ModpackFile> {
    let manager = mods::ModManager::new(game_dir);
    let lock = load_lock(&manager.lock_file()).await;
    let client = ModrinthClient::new(http.clone());

    let mut files = Vec::new();
    for entry in &lock.mods {
        if !manager.mods_dir().join(&entry.file).is_file() {
            continue; // desactivado o ausente: no se promociona al paquete
        }
        let Ok(version) = client.version(&entry.version_id).await else { continue };
        let Some(file) = version.files.iter().find(|f| f.filename == entry.file) else { continue };
        let (Some(sha1), Some(sha512)) = (file.hashes.sha1.clone(), file.hashes.sha512.clone()) else { continue };
        let path = format!("mods/{}", entry.file);
        if validate_relative_path(&path).is_err() {
            continue;
        }
        files.push(ModpackFile {
            path,
            hashes: FileHashes { sha1, sha512 },
            env: Some(FileEnv { client: "required".into(), server: "required".into() }),
            downloads: vec![file.url.clone()],
            file_size: file.size,
        });
    }
    files
}

/// Archivos de la instancia que se empaquetan en `overrides/` (ruta relativa con `/`).
fn collect_bundled(game_dir: &Path, skip: &HashSet<String>) -> Result<Vec<String>, ModpackError> {
    let mut out = Vec::new();
    for dir in EXPORT_DIRS {
        walk(game_dir, &PathBuf::from(dir), skip, &mut out)?;
    }
    for file in EXPORT_FILES {
        if game_dir.join(file).is_file() {
            out.push((*file).to_string());
        }
    }
    out.sort();
    Ok(out)
}

fn walk(root: &Path, relative: &Path, skip: &HashSet<String>, out: &mut Vec<String>) -> Result<(), ModpackError> {
    let dir = root.join(relative);
    let Ok(entries) = std::fs::read_dir(&dir) else { return Ok(()) };
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        let rel = relative.join(entry.file_name());
        if kind.is_dir() {
            walk(root, &rel, skip, out)?;
        } else if kind.is_file() {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if rel_str.ends_with(".part")
                || entry.file_name() == GENERATED_SKIN_PACK
                || skip.contains(&rel_str) || validate_relative_path(&rel_str).is_err() {
                continue;
            }
            out.push(rel_str);
            if out.len() > MAX_EXPORT_FILES {
                return Err(ModpackError::TooLarge);
            }
        }
    }
    Ok(())
}

/// Quita de `options.txt` las referencias al pack de skin del launcher (que no viaja en el paquete).
fn without_generated_skin_pack(options: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in options.lines() {
        // `incompatibleResourcePacks` lo anota el launcher junto a `resourcePacks` (ver `skin_pack.rs`).
        let list = ["resourcePacks:", "incompatibleResourcePacks:"]
            .into_iter()
            .find_map(|key| line.strip_prefix(key).map(|rest| (key, rest)));
        if let Some((key, rest)) = list {
            if let Ok(packs) = serde_json::from_str::<Vec<String>>(rest) {
                let kept: Vec<String> = packs
                    .into_iter()
                    .filter(|p| p != GENERATED_SKIN_PACK && p.strip_prefix("file/") != Some(GENERATED_SKIN_PACK))
                    .collect();
                out.push(format!("{key}{}", serde_json::to_string(&kept).unwrap_or_else(|_| "[]".into())));
                continue;
            }
        }
        if line.strip_prefix("skin:").is_some_and(|v| v.trim() == GENERATED_SKIN_PACK) {
            out.push("skin:Default".to_string());
            continue;
        }
        out.push(line.to_string());
    }
    let mut text = out.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

fn write_package(dest: &Path, index: &ModpackIndex, game_dir: &Path, bundled: &[String]) -> Result<(), ModpackError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("mrpack.part");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&part)?);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file(INDEX_FILE, options)?;
    zip.write_all(serde_json::to_string_pretty(index)?.as_bytes())?;

    for rel in bundled {
        zip.start_file(format!("overrides/{rel}"), options)?;
        if rel == "options.txt" {
            let text = std::fs::read_to_string(game_dir.join(rel)).unwrap_or_default();
            zip.write_all(without_generated_skin_pack(&text).as_bytes())?;
        } else {
            let mut source = std::fs::File::open(game_dir.join(rel))?;
            std::io::copy(&mut source, &mut zip)?;
        }
    }
    zip.finish()?;
    std::fs::rename(&part, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game_with_files() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, data) in [
            ("config/a.toml", "a"),
            ("mods/local.jar", "jar"),
            ("mods/half.jar.part", "x"),
            ("resourcepacks/pack.zip", "z"),
            ("saves/world/level.dat", "secret"),
            ("logs/latest.log", "log"),
            (".disorder/mods.json", "{}"),
            ("options.txt", "o"),
        ] {
            let p = dir.path().join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, data).unwrap();
        }
        dir
    }

    #[test]
    fn options_lose_the_generated_skin_pack_but_keep_everything_else() {
        let options = "fov:1\nresourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\",\"file/mio.zip\"]\nincompatibleResourcePacks:[\"file/DisorderLauncher-skin.zip\"]\nskin:DisorderLauncher-skin.zip\n";
        assert_eq!(
            without_generated_skin_pack(options),
            "fov:1\nresourcePacks:[\"vanilla\",\"file/mio.zip\"]\nincompatibleResourcePacks:[]\nskin:Default\n"
        );
        assert_eq!(without_generated_skin_pack("skin:Faithful.zip\n"), "skin:Faithful.zip\n");
    }

    #[test]
    fn bundles_only_exportable_files() {
        let dir = game_with_files();
        let files = collect_bundled(dir.path(), &HashSet::new()).unwrap();
        assert_eq!(files, ["config/a.toml", "mods/local.jar", "options.txt", "resourcepacks/pack.zip"]);
    }

    #[test]
    fn remote_files_are_not_bundled_twice() {
        let dir = game_with_files();
        let skip = HashSet::from(["mods/local.jar".to_string()]);
        assert!(!collect_bundled(dir.path(), &skip).unwrap().contains(&"mods/local.jar".to_string()));
    }

    #[tokio::test]
    async fn export_then_import_roundtrip_without_network_sources() {
        let dir = game_with_files();
        let dest = dir.path().join("out").join("pack.mrpack");
        let input = ExportInput {
            name: "Mi pack",
            version_id: "1.0.0",
            summary: Some("prueba"),
            minecraft_version: "1.20.1",
            loader: Some((LoaderKind::Fabric, "0.15.0")),
            game_dir: dir.path(),
        };
        let report = export_mrpack(&reqwest::Client::new(), &input, &dest).await.unwrap();
        assert_eq!(report.remote_files, 0);
        assert_eq!(report.bundled_files, 4);

        let index = crate::read_index(&dest).unwrap();
        assert_eq!(index.name, "Mi pack");
        assert_eq!(index.loader().unwrap().unwrap().version, "0.15.0");

        let target = tempfile::tempdir().unwrap();
        let written = crate::apply_overrides(&dest, target.path()).await.unwrap();
        assert_eq!(written, 4);
        assert_eq!(std::fs::read(target.path().join("config/a.toml")).unwrap(), b"a");
        assert!(!target.path().join("saves").exists());
        assert!(!target.path().join("logs").exists());
        assert!(!dest.with_extension("mrpack.part").exists());
    }
}
