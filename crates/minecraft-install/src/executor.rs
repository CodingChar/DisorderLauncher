use crate::error::InstallError;
use crate::plan_builder::build_asset_artifacts;
use crate::{Artifact, InstallPhase, InstallPlan, InstallProgress};
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
        _progress: &Arc<impl Fn(InstallProgress) + Send + Sync>,
        _phase: InstallPhase,
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

        let report = self
            .download_manager
            .download_all(tasks, self.concurrency, |_| {})
            .await?;

        if !report.failed.is_empty() {
            return Err(InstallError::DownloadFailures(report.failed.len()));
        }

        Ok(())
    }
}
