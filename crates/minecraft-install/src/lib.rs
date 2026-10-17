mod error;
mod plan_builder;
mod executor;
mod legacy_assets;
mod rules;

pub use error::*;
pub use plan_builder::*;
pub use executor::*;
pub use legacy_assets::*;

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
    /// Jars de natives (descargados en `libraries/`); se extraen en `natives_dir`.
    pub natives: Vec<Artifact>,
    pub native_extractions: Vec<NativeExtraction>,
    pub natives_dir: PathBuf,
    pub java_major_version: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NativeExtraction {
    pub jar: PathBuf,
    /// Prefijos a omitir al extraer (`extract.exclude` de la libreria).
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPhase {
    Metadata,
    Client,
    Libraries,
    AssetIndex,
    Assets,
    Natives,
    Verify,
}

#[derive(Debug, Clone)]
pub struct InstallProgress {
    pub phase: InstallPhase,
    pub completed: usize,
    pub total: usize,
}
