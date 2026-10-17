use crate::{
    ensure_trusted, validate_component, InstalledLoader, LoaderError, LoaderInstallContext,
    LoaderProvider, LoaderVersion,
};
use download_manager::{DownloadManager, DownloadTask};
use instance_manager::LoaderKind;
use minecraft_launch::LaunchOverlay;
use minecraft_meta::{resolve_library, LoaderProfile};
use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

pub struct FabricProvider {
    http: reqwest::Client,
    meta_url: String,
}

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Debug, Deserialize)]
struct LoaderInfo {
    version: String,
    #[serde(default)]
    stable: bool,
}

impl FabricProvider {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http, meta_url: FABRIC_META_URL.to_string() }
    }

    pub fn with_meta_url(mut self, url: impl Into<String>) -> Self {
        self.meta_url = url.into();
        self
    }

    /// Identificador que usa Fabric para el perfil (y que usamos como carpeta de version).
    pub fn profile_id(minecraft_version: &str, loader_version: &str) -> String {
        format!("fabric-loader-{loader_version}-{minecraft_version}")
    }

    fn profile_path(data_dir: &Path, profile_id: &str) -> PathBuf {
        data_dir.join("versions").join(profile_id).join(format!("{profile_id}.json"))
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, LoaderError> {
        Ok(self.http.get(url).send().await?.error_for_status()?.bytes().await?.to_vec())
    }
}

/// Convierte un perfil en overlay y la lista de descargas pendientes de librerias.
pub fn overlay_from_profile(profile: &LoaderProfile) -> LaunchOverlay {
    LaunchOverlay {
        main_class: Some(profile.main_class.clone()),
        libraries: profile.libraries.clone(),
        arguments: profile.arguments.clone(),
    }
}

pub fn parse_loader_versions(json: &[u8]) -> Result<Vec<LoaderVersion>, LoaderError> {
    let entries: Vec<LoaderEntry> = serde_json::from_slice(json)?;
    Ok(entries
        .into_iter()
        .map(|e| LoaderVersion { version: e.loader.version, stable: e.loader.stable })
        .collect())
}

#[async_trait::async_trait]
impl LoaderProvider for FabricProvider {
    fn kind(&self) -> LoaderKind {
        LoaderKind::Fabric
    }

    async fn versions(&self, minecraft_version: &str) -> Result<Vec<LoaderVersion>, LoaderError> {
        validate_component(minecraft_version)?;
        let url = format!("{}/versions/loader/{minecraft_version}", self.meta_url);
        let versions = parse_loader_versions(&self.get_bytes(&url).await?)?;
        if versions.is_empty() {
            return Err(LoaderError::NoVersionFor(minecraft_version.to_string()));
        }
        Ok(versions)
    }

    async fn install(&self, ctx: &LoaderInstallContext<'_>) -> Result<InstalledLoader, LoaderError> {
        validate_component(ctx.minecraft_version)?;
        validate_component(ctx.loader_version)?;

        let url = format!(
            "{}/versions/loader/{}/{}/profile/json",
            self.meta_url, ctx.minecraft_version, ctx.loader_version
        );
        let bytes = self.get_bytes(&url).await?;
        let profile: LoaderProfile = serde_json::from_slice(&bytes)?;

        let expected_id = Self::profile_id(ctx.minecraft_version, ctx.loader_version);
        if profile.id != expected_id || profile.inherits_from.as_deref() != Some(ctx.minecraft_version) {
            return Err(LoaderError::ProfileMismatch);
        }

        let libraries_dir = ctx.data_dir.join("libraries");
        let mut tasks = Vec::new();
        for lib in &profile.libraries {
            let Some(resolved) = resolve_library(lib) else { continue };
            ensure_trusted(&resolved.url)?;
            tasks.push(DownloadTask {
                url: resolved.url,
                dest: libraries_dir.join(&resolved.path),
                size: resolved.size,
                sha1: resolved.sha1,
            });
        }

        let progress = ctx.progress.clone();
        let report = DownloadManager::new(self.http.clone())
            .download_all(tasks, ctx.concurrency, move |p| progress(p.completed, p.total))
            .await?;
        if !report.failed.is_empty() {
            return Err(LoaderError::DownloadFailures(report.failed.len()));
        }

        // El perfil se guarda al final: su existencia indica una instalacion completa.
        let path = Self::profile_path(ctx.data_dir, &profile.id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = path.with_extension("json.part");
        tokio::fs::write(&part, &bytes).await?;
        tokio::fs::rename(&part, &path).await?;

        Ok(InstalledLoader { overlay: overlay_from_profile(&profile), profile_id: profile.id })
    }

    async fn load_installed(
        &self,
        data_dir: &Path,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<Option<InstalledLoader>, LoaderError> {
        validate_component(minecraft_version)?;
        validate_component(loader_version)?;

        let profile_id = Self::profile_id(minecraft_version, loader_version);
        let Ok(data) = tokio::fs::read(Self::profile_path(data_dir, &profile_id)).await else {
            return Ok(None);
        };
        let Ok(profile) = serde_json::from_slice::<LoaderProfile>(&data) else {
            return Ok(None);
        };

        let libraries_dir = data_dir.join("libraries");
        for lib in &profile.libraries {
            if let Some(resolved) = resolve_library(lib) {
                let present = tokio::fs::metadata(libraries_dir.join(&resolved.path))
                    .await
                    .map(|m| m.is_file())
                    .unwrap_or(false);
                if !present {
                    return Ok(None);
                }
            }
        }
        Ok(Some(InstalledLoader { overlay: overlay_from_profile(&profile), profile_id }))
    }
}
