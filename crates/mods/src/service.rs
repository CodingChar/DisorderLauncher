use crate::local::{split_mod_file, validate_file_name, DISABLED_SUFFIX};
use crate::{
    ensure_trusted_file_url, load_lock, pick_best_version, plan_install, save_lock, LockedMod, ModError,
    ModManager, ModVersion, ModrinthClient, ModrinthSource, PlannedMod, PreviousVersion, ProjectHit,
};
use chrono::Utc;
use download_manager::{DownloadManager, DownloadTask};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

/// Version de Minecraft y loader contra los que se filtra todo.
#[derive(Debug, Clone, Copy)]
pub struct InstanceTarget<'a> {
    pub minecraft_version: &'a str,
    pub loader: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub project_id: String,
    pub title: String,
    pub version_number: String,
    pub file: String,
    /// `true` si se instalo porque otro mod lo necesita.
    pub is_dependency: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub file: String,
    pub project_id: String,
    pub title: String,
    pub current_version: String,
    pub latest_version: String,
}

pub struct ModsService {
    http: reqwest::Client,
    client: ModrinthClient,
}

impl ModsService {
    pub fn new(http: reqwest::Client) -> Self {
        Self { client: ModrinthClient::new(http.clone()), http }
    }

    pub fn with_client(mut self, client: ModrinthClient) -> Self {
        self.client = client;
        self
    }

    fn source(&self, target: InstanceTarget<'_>) -> ModrinthSource {
        ModrinthSource {
            client: self.client.clone(),
            minecraft_version: target.minecraft_version.to_string(),
            loader: target.loader.to_string(),
        }
    }

    pub async fn search(
        &self,
        query: &str,
        target: InstanceTarget<'_>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ProjectHit>, ModError> {
        self.client.search(query, target.minecraft_version, target.loader, limit, offset).await
    }

    /// Instala un mod con sus dependencias obligatorias. Nada se escribe si falta alguna.
    pub async fn install(
        &self,
        manager: &ModManager,
        project_id: &str,
        target: InstanceTarget<'_>,
        concurrency: usize,
    ) -> Result<Vec<InstalledMod>, ModError> {
        let tracked = tracked_projects(manager).await;
        let plan = plan_install(&self.source(target), project_id, &tracked).await?;
        self.install_plan(manager, plan, concurrency).await
    }

    pub async fn install_plan(
        &self,
        manager: &ModManager,
        plan: Vec<PlannedMod>,
        concurrency: usize,
    ) -> Result<Vec<InstalledMod>, ModError> {
        let mut prepared = Vec::new();
        for planned in &plan {
            prepared.push(prepare_download(manager, &planned.version)?);
        }
        let tasks = prepared.iter().map(|p| p.task.clone()).collect();
        download(&self.http, tasks, concurrency).await?;

        let mut installed = Vec::new();
        for (planned, prepared) in plan.into_iter().zip(prepared) {
            manager
                .record(LockedMod {
                    file: prepared.file_name.clone(),
                    project_id: planned.project_id.clone(),
                    version_id: planned.version.id.clone(),
                    version_number: planned.version.version_number.clone(),
                    title: planned.title.clone(),
                    sha1: prepared.sha1,
                    installed_at: Utc::now(),
                    dependency_of: planned.dependency_of.clone(),
                    previous: None,
                })
                .await?;
            installed.push(InstalledMod {
                project_id: planned.project_id,
                title: planned.title,
                version_number: planned.version.version_number,
                file: prepared.file_name,
                is_dependency: planned.dependency_of.is_some(),
            });
        }
        Ok(installed)
    }

    /// Registra como mods de Modrinth los archivos ya presentes en la instancia que Modrinth reconoce
    /// por su hash (p. ej. tras importar un modpack). Asi pueden actualizarse y volver atras como
    /// los instalados desde el launcher. `files` son (nombre en `mods/`, SHA-1). Devuelve cuantos se
    /// registraron; lo que Modrinth no conoce se deja como mod local.
    pub async fn register_known(&self, manager: &ModManager, files: &[(String, String)]) -> Result<usize, ModError> {
        let present: Vec<&(String, String)> = files
            .iter()
            .filter(|(name, _)| validate_file_name(name).is_ok() && manager.mods_dir().join(name).is_file())
            .collect();
        let hashes: Vec<String> = present.iter().map(|(_, sha1)| sha1.to_lowercase()).collect();
        let known = self.client.versions_by_sha1(&hashes).await?;
        let projects: Vec<String> = {
            let mut ids: Vec<String> = known.values().map(|v| v.project_id.clone()).collect();
            ids.sort();
            ids.dedup();
            ids
        };
        let titles = self.client.project_titles(&projects).await.unwrap_or_default();

        let mut registered = 0;
        for (name, sha1) in present {
            let Some(version) = known.get(&sha1.to_lowercase()) else { continue };
            manager
                .record(LockedMod {
                    file: name.clone(),
                    project_id: version.project_id.clone(),
                    version_id: version.id.clone(),
                    version_number: version.version_number.clone(),
                    title: titles.get(&version.project_id).cloned().unwrap_or_else(|| version.name.clone()),
                    sha1: sha1.to_lowercase(),
                    installed_at: Utc::now(),
                    dependency_of: None,
                    previous: None,
                })
                .await?;
            registered += 1;
        }
        Ok(registered)
    }

    /// Mods instalados desde Modrinth que tienen una version mas nueva compatible.
    pub async fn check_updates(
        &self,
        manager: &ModManager,
        target: InstanceTarget<'_>,
    ) -> Result<Vec<UpdateInfo>, ModError> {
        let lock = load_lock(&manager.lock_path()).await;
        let source = self.source(target);
        let mut updates = Vec::new();

        for entry in &lock.mods {
            if find_on_disk(manager, &entry.file).is_none() {
                continue;
            }
            let versions = crate::VersionSource::compatible_versions(&source, &entry.project_id).await?;
            if let Some(best) = pick_best_version(&versions) {
                if best.id != entry.version_id {
                    updates.push(UpdateInfo {
                        file: entry.file.clone(),
                        project_id: entry.project_id.clone(),
                        title: entry.title.clone(),
                        current_version: entry.version_number.clone(),
                        latest_version: best.version_number.clone(),
                    });
                }
            }
        }
        Ok(updates)
    }

    /// Actualiza un mod guardando la version anterior para poder volver atras.
    pub async fn update(
        &self,
        manager: &ModManager,
        file: &str,
        target: InstanceTarget<'_>,
    ) -> Result<InstalledMod, ModError> {
        let (base, _) = split_mod_file(file).ok_or(ModError::InvalidFileName)?;
        let mut lock = load_lock(&manager.lock_path()).await;
        let current = lock.find_file(base).cloned().ok_or(ModError::NotFound)?;
        let (old_path, was_enabled) = find_on_disk(manager, base).ok_or(ModError::NotFound)?;

        let versions = crate::VersionSource::compatible_versions(&self.source(target), &current.project_id).await?;
        let best = pick_best_version(&versions).cloned().ok_or(ModError::NotCompatible(current.title.clone()))?;
        if best.id == current.version_id {
            return Err(ModError::NotFound);
        }
        let prepared = prepare_download(manager, &best)?;

        // 1. Apartar la version actual (conserva su nombre original en la copia).
        fs::create_dir_all(manager.backup_dir()).await?;
        let backup_name = format!("{}--{}", current.version_id, base);
        let backup_path = manager.backup_dir().join(&backup_name);
        fs::rename(&old_path, &backup_path).await?;

        // 2. Descargar la nueva; si falla, devolver la anterior.
        if let Err(e) = download(&self.http, vec![prepared.task.clone()], 1).await {
            let _ = fs::rename(&backup_path, &old_path).await;
            return Err(e);
        }
        if !was_enabled {
            let active = manager.mods_dir().join(&prepared.file_name);
            fs::rename(&active, manager.mods_dir().join(format!("{}{DISABLED_SUFFIX}", prepared.file_name))).await?;
        }

        // 3. Registrar, descartando una copia anterior mas vieja.
        if let Some(older) = &current.previous {
            let _ = fs::remove_file(manager.backup_dir().join(&older.file)).await;
        }
        lock.upsert(LockedMod {
            file: prepared.file_name.clone(),
            project_id: current.project_id.clone(),
            version_id: best.id.clone(),
            version_number: best.version_number.clone(),
            title: current.title.clone(),
            sha1: prepared.sha1,
            installed_at: Utc::now(),
            dependency_of: current.dependency_of.clone(),
            previous: Some(PreviousVersion {
                file: backup_name,
                version_id: current.version_id,
                version_number: current.version_number,
                sha1: current.sha1,
            }),
        });
        save_lock(&manager.lock_path(), &lock).await?;

        Ok(InstalledMod {
            project_id: current.project_id,
            title: current.title,
            version_number: best.version_number,
            file: prepared.file_name,
            is_dependency: current.dependency_of.is_some(),
        })
    }

    /// Restaura la version que habia antes de la ultima actualizacion.
    pub async fn rollback(&self, manager: &ModManager, file: &str) -> Result<(), ModError> {
        let (base, _) = split_mod_file(file).ok_or(ModError::InvalidFileName)?;
        let mut lock = load_lock(&manager.lock_path()).await;
        let current = lock.find_file(base).cloned().ok_or(ModError::NotFound)?;
        let previous = current.previous.clone().ok_or(ModError::NothingToRollback)?;

        let backup_path = manager.backup_dir().join(&previous.file);
        if !backup_path.is_file() {
            return Err(ModError::NothingToRollback);
        }
        // El nombre original va tras el primer `--` (los ids de version no lo contienen).
        let original = previous.file.split_once("--").map(|(_, rest)| rest).ok_or(ModError::NothingToRollback)?;
        validate_file_name(original)?;

        let (current_path, was_enabled) = find_on_disk(manager, base).ok_or(ModError::NotFound)?;
        fs::remove_file(&current_path).await?;

        let restored = if was_enabled {
            original.to_string()
        } else {
            format!("{original}{DISABLED_SUFFIX}")
        };
        fs::rename(&backup_path, manager.mods_dir().join(restored)).await?;

        lock.remove_file(base);
        lock.upsert(LockedMod {
            file: original.to_string(),
            version_id: previous.version_id,
            version_number: previous.version_number,
            sha1: previous.sha1,
            previous: None,
            ..current
        });
        save_lock(&manager.lock_path(), &lock).await
    }
}

struct PreparedDownload {
    file_name: String,
    sha1: String,
    task: DownloadTask,
}

fn prepare_download(manager: &ModManager, version: &ModVersion) -> Result<PreparedDownload, ModError> {
    let file = version.primary_file().ok_or(ModError::NoFile)?;
    ensure_trusted_file_url(&file.url)?;
    validate_file_name(&file.filename)?;
    if !file.filename.ends_with(".jar") {
        return Err(ModError::InvalidFileName);
    }
    let sha1 = file.hashes.sha1.clone().filter(|h| !h.is_empty()).ok_or(ModError::NoFile)?.to_lowercase();
    Ok(PreparedDownload {
        file_name: file.filename.clone(),
        sha1: sha1.clone(),
        task: DownloadTask {
            url: file.url.clone(),
            dest: manager.mods_dir().join(&file.filename),
            size: (file.size > 0).then_some(file.size),
            sha1: Some(sha1),
        },
    })
}

async fn download(http: &reqwest::Client, tasks: Vec<DownloadTask>, concurrency: usize) -> Result<(), ModError> {
    let report = DownloadManager::new(http.clone()).download_all(tasks, concurrency.max(1), |_| {}).await?;
    if let Some(failure) = report.failed.first() {
        tracing::error!(error = %failure.error, "fallo la descarga de un mod");
        return Err(ModError::NoFile);
    }
    Ok(())
}

/// `(ruta, activo)` del archivo del mod en disco, activo o desactivado.
fn find_on_disk(manager: &ModManager, base: &str) -> Option<(PathBuf, bool)> {
    let enabled = manager.mods_dir().join(base);
    if enabled.is_file() {
        return Some((enabled, true));
    }
    let disabled = manager.mods_dir().join(format!("{base}{DISABLED_SUFFIX}"));
    disabled.is_file().then_some((disabled, false))
}

/// Proyectos de Modrinth que siguen presentes en disco (para no reinstalar dependencias).
async fn tracked_projects(manager: &ModManager) -> HashSet<String> {
    load_lock(&manager.lock_path())
        .await
        .mods
        .into_iter()
        .filter(|m| find_on_disk(manager, &m.file).is_some())
        .map(|m| m.project_id)
        .collect()
}
