use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Artifact {
    pub url: String,
    pub path: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub version: String,
    pub client: Artifact,
    pub libraries: Vec<Artifact>,
    pub asset_index: Artifact,
    pub assets: Vec<Artifact>,
    pub natives: Vec<Artifact>,
    pub java_major_version: Option<u32>,
}
