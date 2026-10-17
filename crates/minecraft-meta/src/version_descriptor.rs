use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDescriptor {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub main_class: String,
    #[serde(default)]
    pub minimum_launcher_version: u32,
    #[serde(default)]
    pub compliance_level: u32,
    pub downloads: VersionDownloads,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndexInfo,
    pub assets: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default, rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub java_version: Option<JavaVersionInfo>,
    #[serde(default)]
    pub logging: Option<HashMap<String, LoggingConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadInfo,
    #[serde(default)]
    pub server: Option<DownloadInfo>,
    #[serde(default)]
    pub client_mappings: Option<DownloadInfo>,
    #[serde(default)]
    pub server_mappings: Option<DownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexInfo {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<ArgumentValue>,
    #[serde(default)]
    pub jvm: Vec<ArgumentValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Simple(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValueInner,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValueInner {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub extract: Option<ExtractConfig>,
    /// Librerias estilo Maven (loaders): URL base del repositorio.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<LibraryArtifact>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, LibraryArtifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionInfo {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub argument: String,
    pub file: LoggingFile,
    #[serde(rename = "type")]
    pub log_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    /// Indices antiguos: los assets se exponen por nombre en `assets/virtual/<id>/`.
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    /// Versiones muy antiguas: los assets se esperan en `<gameDir>/resources/`.
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}
