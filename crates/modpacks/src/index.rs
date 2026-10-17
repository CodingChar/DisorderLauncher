use crate::ModpackError;
use instance_manager::LoaderKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path};
use url::Url;

pub const INDEX_FILE: &str = "modrinth.index.json";
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_FILES: usize = 5000;

/// Hosts permitidos por la especificacion de `.mrpack` para `downloads`.
const TRUSTED_DOWNLOAD_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFile {
    pub path: String,
    pub hashes: FileHashes,
    #[serde(default)]
    pub env: Option<FileEnv>,
    pub downloads: Vec<String>,
    #[serde(default)]
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEnv {
    pub client: String,
    pub server: String,
}

/// Loader que pide el modpack y su version exacta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredLoader {
    pub kind: LoaderKind,
    pub version: String,
}

impl ModpackIndex {
    pub fn minecraft_version(&self) -> Result<&str, ModpackError> {
        self.dependencies.get("minecraft").map(String::as_str).ok_or(ModpackError::NotAModpack)
    }

    /// `None` para modpacks vanilla. Los loaders aun sin soporte devuelven error legible.
    pub fn loader(&self) -> Result<Option<RequiredLoader>, ModpackError> {
        for (key, kind) in [("fabric-loader", LoaderKind::Fabric), ("quilt-loader", LoaderKind::Quilt)] {
            if let Some(version) = self.dependencies.get(key) {
                return Ok(Some(RequiredLoader { kind, version: version.clone() }));
            }
        }
        for (key, name) in [("forge", "Forge"), ("neoforge", "NeoForge")] {
            if self.dependencies.contains_key(key) {
                return Err(ModpackError::UnsupportedLoader(name.to_string()));
            }
        }
        Ok(None)
    }

    /// Archivos que se descargan en el cliente (se omiten los marcados como no soportados).
    pub fn client_files(&self) -> impl Iterator<Item = &ModpackFile> {
        self.files.iter().filter(|f| f.env.as_ref().map(|e| e.client != "unsupported").unwrap_or(true))
    }
}

/// Lee y valida `modrinth.index.json` de un `.mrpack`. Bloqueante.
pub fn read_index(mrpack: &Path) -> Result<ModpackIndex, ModpackError> {
    let file = std::fs::File::open(mrpack)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| ModpackError::NotAModpack)?;
    let entry = zip.by_name(INDEX_FILE).map_err(|_| ModpackError::NotAModpack)?;
    if entry.size() > MAX_INDEX_BYTES {
        return Err(ModpackError::TooLarge);
    }
    let mut data = Vec::new();
    entry.take(MAX_INDEX_BYTES).read_to_end(&mut data)?;
    parse_index(&data)
}

pub fn parse_index(json: &[u8]) -> Result<ModpackIndex, ModpackError> {
    let index: ModpackIndex = serde_json::from_slice(json).map_err(|_| ModpackError::NotAModpack)?;
    validate_index(&index)?;
    Ok(index)
}

fn validate_index(index: &ModpackIndex) -> Result<(), ModpackError> {
    if index.format_version != 1 {
        return Err(ModpackError::UnsupportedFormat(format!("version {}", index.format_version)));
    }
    if index.game != "minecraft" {
        return Err(ModpackError::UnsupportedFormat(index.game.clone()));
    }
    if index.files.len() > MAX_FILES {
        return Err(ModpackError::TooLarge);
    }
    index.minecraft_version()?;

    for file in &index.files {
        validate_relative_path(&file.path)?;
        if file.hashes.sha1.trim().is_empty() {
            return Err(ModpackError::MissingHash(file.path.clone()));
        }
        if file.downloads.is_empty() {
            return Err(ModpackError::UntrustedSource(file.path.clone()));
        }
        for url in &file.downloads {
            ensure_trusted_download(url)?;
        }
    }
    Ok(())
}

/// Ruta relativa dentro de la carpeta del juego: sin `..`, sin raiz, sin unidades, sin
/// la carpeta interna del launcher.
pub fn validate_relative_path(path: &str) -> Result<(), ModpackError> {
    let p = Path::new(path);
    let ok = !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && p.components().all(|c| matches!(c, Component::Normal(_)))
        && !path.starts_with(".disorder");
    if ok { Ok(()) } else { Err(ModpackError::UnsafePath(path.chars().take(80).collect())) }
}

fn ensure_trusted_download(url: &str) -> Result<(), ModpackError> {
    let parsed = Url::parse(url).map_err(|_| ModpackError::UntrustedSource(url.chars().take(60).collect()))?;
    let ok = parsed.scheme() == "https" && parsed.host_str().is_some_and(|h| TRUSTED_DOWNLOAD_HOSTS.contains(&h));
    if ok { Ok(()) } else { Err(ModpackError::UntrustedSource(parsed.host_str().unwrap_or("?").to_string())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index_json(files: &str, deps: &str) -> String {
        format!(
            r#"{{"formatVersion":1,"game":"minecraft","versionId":"1.0.0","name":"Mi pack","summary":"x","files":[{files}],"dependencies":{{{deps}}}}}"#
        )
    }

    fn file(path: &str, url: &str, env: &str) -> String {
        format!(
            r#"{{"path":"{path}","hashes":{{"sha1":"aa","sha512":"bb"}},"env":{env},"downloads":["{url}"],"fileSize":10}}"#
        )
    }

    const OK_URL: &str = "https://cdn.modrinth.com/data/x/versions/y/a.jar";

    #[test]
    fn parses_a_valid_index() {
        let json = index_json(&file("mods/a.jar", OK_URL, r#"{"client":"required","server":"required"}"#), r#""minecraft":"1.20.1","fabric-loader":"0.15.0""#);
        let index = parse_index(json.as_bytes()).unwrap();
        assert_eq!(index.name, "Mi pack");
        assert_eq!(index.minecraft_version().unwrap(), "1.20.1");
        assert_eq!(index.loader().unwrap(), Some(RequiredLoader { kind: LoaderKind::Fabric, version: "0.15.0".into() }));
    }

    #[test]
    fn unsupported_loaders_have_readable_errors_and_vanilla_is_none() {
        let forge = parse_index(index_json("", r#""minecraft":"1.20.1","forge":"47.2.0""#).as_bytes()).unwrap();
        assert!(matches!(forge.loader(), Err(ModpackError::UnsupportedLoader(n)) if n == "Forge"));
        let vanilla = parse_index(index_json("", r#""minecraft":"1.20.1""#).as_bytes()).unwrap();
        assert_eq!(vanilla.loader().unwrap(), None);
    }

    #[test]
    fn rejects_unsafe_paths() {
        for bad in ["../evil.jar", "/abs/a.jar", "C:/a.jar", "mods/../../a.jar", ".disorder/mods.json", ""] {
            let json = index_json(&file(bad, OK_URL, "null"), r#""minecraft":"1.20.1""#);
            assert!(matches!(parse_index(json.as_bytes()), Err(ModpackError::UnsafePath(_))), "{bad}");
        }
    }

    #[test]
    fn backslashes_are_rejected_in_paths() {
        assert!(validate_relative_path(r"mods\a.jar").is_err());
        assert!(validate_relative_path("mods/a.jar").is_ok());
    }

    #[test]
    fn rejects_untrusted_download_hosts() {
        for bad in ["http://cdn.modrinth.com/a.jar", "https://evil.example.com/a.jar", "https://cdn.modrinth.com.evil.io/a.jar", "ftp://x/a"] {
            let json = index_json(&file("mods/a.jar", bad, "null"), r#""minecraft":"1.20.1""#);
            assert!(matches!(parse_index(json.as_bytes()), Err(ModpackError::UntrustedSource(_))), "{bad}");
        }
    }

    #[test]
    fn rejects_bad_format_and_game() {
        let v2 = index_json("", r#""minecraft":"1.20.1""#).replace("\"formatVersion\":1", "\"formatVersion\":2");
        assert!(matches!(parse_index(v2.as_bytes()), Err(ModpackError::UnsupportedFormat(_))));
        let other = index_json("", r#""minecraft":"1.20.1""#).replace("\"game\":\"minecraft\"", "\"game\":\"other\"");
        assert!(matches!(parse_index(other.as_bytes()), Err(ModpackError::UnsupportedFormat(_))));
        assert!(matches!(parse_index(b"nope"), Err(ModpackError::NotAModpack)));
        assert!(matches!(parse_index(index_json("", "").as_bytes()), Err(ModpackError::NotAModpack)), "sin version de Minecraft");
    }

    #[test]
    fn client_files_skip_unsupported() {
        let files = format!(
            "{},{}",
            file("mods/a.jar", OK_URL, r#"{"client":"required","server":"required"}"#),
            file("mods/server-only.jar", OK_URL, r#"{"client":"unsupported","server":"required"}"#)
        );
        let index = parse_index(index_json(&files, r#""minecraft":"1.20.1""#).as_bytes()).unwrap();
        let names: Vec<_> = index.client_files().map(|f| f.path.as_str()).collect();
        assert_eq!(names, ["mods/a.jar"]);
    }
}
