use crate::error::InstallError;
use crate::plan_builder::build_asset_artifacts;
use crate::{Artifact, InstallPhase, InstallPlan, InstallProgress};
use download_manager::archive::{extract_zip, ExtractOptions};
use download_manager::{DownloadManager, DownloadTask};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

pub struct InstallExecutor {
    download_manager: DownloadManager,
    concurrency: usize,
}

impl InstallExecutor {
    pub fn new(http: reqwest::Client, concurrency: usize) -> Self {
        Self {
            download_manager: DownloadManager::new(http),
            concurrency,
        }
    }

    pub async fn execute<F>(
        &self,
        plan: &mut InstallPlan,
        game_dir: &Path,
        progress: F,
    ) -> Result<(), InstallError>
    where
        F: Fn(InstallProgress) + Send + Sync + 'static,
    {
        let progress = Arc::new(progress);

        progress(InstallProgress {
            phase: InstallPhase::Client,
            completed: 0,
            total: 1,
        });
        self.download_artifacts(std::slice::from_ref(&plan.client), &progress, InstallPhase::Client)
            .await?;

        progress(InstallProgress {
            phase: InstallPhase::Libraries,
            completed: 0,
            total: plan.libraries.len(),
        });
        self.download_artifacts(&plan.libraries, &progress, InstallPhase::Libraries)
            .await?;

        progress(InstallProgress {
            phase: InstallPhase::AssetIndex,
            completed: 0,
            total: 1,
        });
        self.download_artifacts(
            std::slice::from_ref(&plan.asset_index),
            &progress,
            InstallPhase::AssetIndex,
        )
        .await?;

        let asset_index_path = &plan.asset_index.path;
        if asset_index_path.exists() {
            let data = fs::read_to_string(asset_index_path).await?;
            let index: minecraft_meta::AssetIndex = serde_json::from_str(&data)?;
            plan.assets = build_asset_artifacts(&index, game_dir);

            progress(InstallProgress {
                phase: InstallPhase::Assets,
                completed: 0,
                total: plan.assets.len(),
            });
            self.download_artifacts(&plan.assets, &progress, InstallPhase::Assets)
                .await?;
        }

        if !plan.natives.is_empty() {
            progress(InstallProgress {
                phase: InstallPhase::Natives,
                completed: 0,
                total: plan.natives.len(),
            });
            self.download_artifacts(&plan.natives, &progress, InstallPhase::Natives)
                .await?;
            extract_natives(plan).await?;
        }

        progress(InstallProgress {
            phase: InstallPhase::Verify,
            completed: 0,
            total: 1,
        });

        Ok(())
    }

    async fn download_artifacts(
        &self,
        artifacts: &[Artifact],
        progress: &Arc<impl Fn(InstallProgress) + Send + Sync + 'static>,
        phase: InstallPhase,
    ) -> Result<(), InstallError> {
        let tasks: Vec<DownloadTask> = artifacts
            .iter()
            .map(|a| DownloadTask {
                url: a.url.clone(),
                dest: a.path.clone(),
                size: a.size,
                sha1: a.sha1.clone(),
            })
            .collect();

        let progress = progress.clone();
        let report = self
            .download_manager
            .download_all(tasks, self.concurrency, move |p| {
                progress(InstallProgress {
                    phase,
                    completed: p.completed,
                    total: p.total,
                })
            })
            .await?;

        if !report.failed.is_empty() {
            return Err(InstallError::DownloadFailures(report.failed.len()));
        }

        Ok(())
    }
}

/// Extrae los jars de natives en el directorio de natives de la version.
/// Se reextrae siempre: es barato y evita DLL obsoletas tras una reparacion.
pub async fn extract_natives(plan: &InstallPlan) -> Result<(), InstallError> {
    let natives_dir = plan.natives_dir.clone();
    let extractions = plan.native_extractions.clone();

    tokio::task::spawn_blocking(move || -> Result<(), InstallError> {
        for item in extractions {
            let options = ExtractOptions {
                exclude_prefixes: item.exclude,
                ..ExtractOptions::default()
            };
            extract_zip(&item.jar, &natives_dir, &options)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| InstallError::Io(std::io::Error::other(e.to_string())))?
}
