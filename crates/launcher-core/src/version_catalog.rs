//! Catalogo de versiones de Minecraft que se pueden elegir al crear una instancia.
//!
//! - Con red: el manifiesto de Mojang (todas las versiones: estables, snapshots, beta y alpha),
//!   filtrado por lo que el loader elegido realmente soporta (Fabric, Quilt, Forge, NeoForge).
//! - Sin red: lo ultimo que se descargo (cache) y, si tampoco hay, las versiones ya instaladas.

use crate::{Launcher, LauncherError};
use instance_manager::LoaderKind;
use minecraft_meta::{VersionEntry, VersionType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const LOADER_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogSource {
    /// Lista recien descargada.
    Online,
    /// Sin red: ultima lista descargada.
    Cached,
    /// Sin red ni cache: solo lo que ya esta instalado en este equipo.
    Installed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCatalog {
    pub versions: Vec<VersionEntry>,
    pub source: CatalogSource,
}

fn loader_cache_file(root: &Path, kind: LoaderKind) -> PathBuf {
    root.join("cache").join(format!("loader_games_{}.json", format!("{kind:?}").to_lowercase()))
}

#[derive(Deserialize)]
struct InstalledDescriptor {
    id: String,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(rename = "inheritsFrom", default)]
    inherits_from: Option<String>,
    #[serde(rename = "releaseTime", default)]
    release_time: Option<String>,
}

fn version_type(raw: Option<&str>) -> VersionType {
    match raw {
        Some("snapshot") => VersionType::Snapshot,
        Some("old_beta") => VersionType::OldBeta,
        Some("old_alpha") => VersionType::OldAlpha,
        _ => VersionType::Release,
    }
}

/// ¿El id de un perfil guardado pertenece a este loader?
fn profile_matches(kind: LoaderKind, id: &str) -> bool {
    let id = id.to_lowercase();
    match kind {
        LoaderKind::Fabric => id.starts_with("fabric-loader-"),
        LoaderKind::Quilt => id.starts_with("quilt-loader-"),
        LoaderKind::NeoForge => id.contains("neoforge"),
        LoaderKind::Forge => id.contains("forge") && !id.contains("neoforge"),
        LoaderKind::Vanilla => false,
    }
}

/// Versiones de Minecraft ya instaladas (con el loader pedido, si lo hay), de la mas reciente a la mas antigua.
pub(crate) fn installed_versions(minecraft_dir: &Path, kind: LoaderKind) -> Vec<VersionEntry> {
    let mut descriptors = Vec::new();
    if let Ok(dir) = std::fs::read_dir(minecraft_dir.join("versions")) {
        for entry in dir.flatten() {
            let id = entry.file_name().to_string_lossy().to_string();
            let Ok(raw) = std::fs::read(entry.path().join(format!("{id}.json"))) else { continue };
            if let Ok(d) = serde_json::from_slice::<InstalledDescriptor>(&raw) {
                descriptors.push(d);
            }
        }
    }
    let vanilla: Vec<&InstalledDescriptor> = descriptors.iter().filter(|d| d.inherits_from.is_none()).collect();
    let wanted: HashSet<String> = match kind {
        LoaderKind::Vanilla => vanilla.iter().map(|d| d.id.clone()).collect(),
        // Para un loader solo cuentan las versiones sobre las que ya hay un perfil instalado.
        _ => descriptors
            .iter()
            .filter(|d| profile_matches(kind, &d.id))
            .filter_map(|d| d.inherits_from.clone())
            .collect(),
    };
    let mut out: Vec<VersionEntry> = vanilla
        .into_iter()
        .filter(|d| wanted.contains(&d.id))
        .map(|d| {
            let when = d.release_time.clone().unwrap_or_default();
            VersionEntry {
                id: d.id.clone(),
                version_type: version_type(d.kind.as_deref()),
                url: String::new(),
                time: when.clone(),
                release_time: when,
                sha1: String::new(),
                compliance_level: 0,
            }
        })
        .collect();
    out.sort_by(|a, b| b.release_time.cmp(&a.release_time));
    out
}

/// Version del loader ya instalada sobre `minecraft_version` (para crear instancias sin red).
pub(crate) fn installed_loader_version(minecraft_dir: &Path, kind: LoaderKind, minecraft_version: &str) -> Option<String> {
    let dir = std::fs::read_dir(minecraft_dir.join("versions")).ok()?;
    let mut found: Vec<String> = dir
        .flatten()
        .filter_map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let raw = std::fs::read(e.path().join(format!("{id}.json"))).ok()?;
            let d: InstalledDescriptor = serde_json::from_slice(&raw).ok()?;
            if d.inherits_from.as_deref() != Some(minecraft_version) || !profile_matches(kind, &id) {
                return None;
            }
            match kind {
                LoaderKind::Fabric => id.strip_prefix("fabric-loader-")?.strip_suffix(&format!("-{minecraft_version}")).map(str::to_string),
                LoaderKind::Quilt => id.strip_prefix("quilt-loader-")?.strip_suffix(&format!("-{minecraft_version}")).map(str::to_string),
                LoaderKind::Forge => id.strip_prefix(&format!("{minecraft_version}-forge-")).map(str::to_string),
                LoaderKind::NeoForge => id.strip_prefix("neoforge-").map(str::to_string),
                LoaderKind::Vanilla => None,
            }
        })
        .collect();
    found.sort();
    found.pop()
}

impl Launcher {
    /// Versiones de Minecraft disponibles para un tipo de instancia (ver el modulo).
    pub async fn version_catalog(&self, kind: LoaderKind) -> Result<VersionCatalog, LauncherError> {
        let minecraft_dir = self.paths().minecraft_dir();
        let installed = || VersionCatalog { versions: installed_versions(&minecraft_dir, kind), source: CatalogSource::Installed };

        if kind == LoaderKind::Vanilla {
            return Ok(match self.load_manifest_with_source().await {
                Ok((manifest, cached)) => VersionCatalog {
                    versions: manifest.versions,
                    source: if cached { CatalogSource::Cached } else { CatalogSource::Online },
                },
                Err(_) => installed(),
            });
        }

        let provider = self.loader_provider(kind)?;
        let cache = loader_cache_file(self.paths().root(), kind);
        // Las dos consultas son independientes: se hacen a la vez para esperar solo la mas lenta.
        let (manifest, games) = tokio::join!(
            self.load_manifest_with_source(),
            tokio::time::timeout(LOADER_TIMEOUT, provider.game_versions())
        );
        let Ok((manifest, manifest_cached)) = manifest else {
            return Ok(installed());
        };
        let (supported, loader_cached): (Vec<String>, bool) = match games {
            Ok(Ok(list)) => {
                if let Some(dir) = cache.parent() {
                    let _ = tokio::fs::create_dir_all(dir).await;
                }
                if let Ok(json) = serde_json::to_vec(&list) {
                    let _ = tokio::fs::write(&cache, json).await;
                }
                (list, false)
            }
            _ => match tokio::fs::read(&cache).await.ok().and_then(|raw| serde_json::from_slice(&raw).ok()) {
                Some(list) => (list, true),
                None => return Ok(installed()),
            },
        };

        let supported: HashSet<String> = supported.into_iter().collect();
        let versions = manifest.versions.into_iter().filter(|v| supported.contains(&v.id)).collect();
        let source = if manifest_cached || loader_cached { CatalogSource::Cached } else { CatalogSource::Online };
        Ok(VersionCatalog { versions, source })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_version(root: &Path, id: &str, json: &str) {
        let dir = root.join("versions").join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{id}.json")), json).unwrap();
    }

    #[test]
    fn installed_vanilla_and_loader_versions_are_listed_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        write_version(dir.path(), "1.12.2", r#"{"id":"1.12.2","type":"release","releaseTime":"2017-09-18T08:39:46+00:00"}"#);
        write_version(dir.path(), "1.20.1", r#"{"id":"1.20.1","type":"release","releaseTime":"2023-06-12T13:25:51+00:00"}"#);
        write_version(dir.path(), "b1.7.3", r#"{"id":"b1.7.3","type":"old_beta","releaseTime":"2011-07-08T00:00:00+00:00"}"#);
        write_version(dir.path(), "fabric-loader-0.15.0-1.20.1", r#"{"id":"fabric-loader-0.15.0-1.20.1","inheritsFrom":"1.20.1"}"#);

        let vanilla = installed_versions(dir.path(), LoaderKind::Vanilla);
        let ids: Vec<_> = vanilla.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, ["1.20.1", "1.12.2", "b1.7.3"]);
        assert_eq!(vanilla[2].version_type, VersionType::OldBeta);
        let fabric: Vec<_> = installed_versions(dir.path(), LoaderKind::Fabric).into_iter().map(|v| v.id).collect();
        assert_eq!(fabric, ["1.20.1"]);
        assert!(installed_versions(dir.path(), LoaderKind::Quilt).is_empty());
    }

    #[test]
    fn installed_loader_version_is_read_from_the_profile_id() {
        let dir = tempfile::tempdir().unwrap();
        write_version(dir.path(), "fabric-loader-0.15.0-1.20.1", r#"{"id":"fabric-loader-0.15.0-1.20.1","inheritsFrom":"1.20.1"}"#);
        write_version(dir.path(), "1.12.2-forge-14.23.5.2860", r#"{"id":"1.12.2-forge-14.23.5.2860","inheritsFrom":"1.12.2"}"#);
        write_version(dir.path(), "neoforge-21.1.172", r#"{"id":"neoforge-21.1.172","inheritsFrom":"1.21.1"}"#);
        assert_eq!(installed_loader_version(dir.path(), LoaderKind::Fabric, "1.20.1").as_deref(), Some("0.15.0"));
        assert_eq!(installed_loader_version(dir.path(), LoaderKind::Forge, "1.12.2").as_deref(), Some("14.23.5.2860"));
        assert_eq!(installed_loader_version(dir.path(), LoaderKind::NeoForge, "1.21.1").as_deref(), Some("21.1.172"));
        assert_eq!(installed_loader_version(dir.path(), LoaderKind::Fabric, "1.19").as_deref(), None);
    }

    #[test]
    fn forge_and_neoforge_profiles_are_told_apart() {
        assert!(profile_matches(LoaderKind::Forge, "1.12.2-forge-14.23.5.2860"));
        assert!(!profile_matches(LoaderKind::Forge, "neoforge-21.1.172"));
        assert!(profile_matches(LoaderKind::NeoForge, "neoforge-21.1.172"));
    }
}
