use crate::error::MetaError;
use crate::manifest::{VersionManifest, VERSION_MANIFEST_URL};
use crate::version_descriptor::VersionDescriptor;
use std::path::Path;
use tokio::fs;

pub async fn fetch_manifest(http: &reqwest::Client) -> Result<VersionManifest, MetaError> {
    fetch_manifest_from(http, VERSION_MANIFEST_URL).await
}

pub async fn fetch_manifest_from(
    http: &reqwest::Client,
    url: &str,
) -> Result<VersionManifest, MetaError> {
    let manifest = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(manifest)
}

pub async fn load_cached_manifest(path: &Path) -> Result<VersionManifest, MetaError> {
    let data = fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&data)?)
}

pub async fn save_manifest_cache(
    manifest: &VersionManifest,
    path: &Path,
) -> Result<(), MetaError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_string_pretty(manifest)?;
    fs_safe::write_atomic(path, data).await?;
    Ok(())
}

pub async fn fetch_version_descriptor(
    http: &reqwest::Client,
    url: &str,
) -> Result<VersionDescriptor, MetaError> {
    let descriptor = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(descriptor)
}

pub async fn load_cached_version_descriptor(
    path: &Path,
) -> Result<VersionDescriptor, MetaError> {
    let data = fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&data)?)
}
