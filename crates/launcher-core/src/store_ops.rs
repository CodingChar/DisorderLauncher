//! Almacen comun de artefactos: incorpora los mods de cada instancia para que las copias
//! identicas compartan los mismos bytes en disco. La logica vive en el crate `artifact-store`.

use crate::{InstallProgressEvent, Launcher, LauncherError};
use artifact_store::{adopt_instance, AdoptReport, ArtifactError, ArtifactStore};
use instance_manager::InstanceRepository;
use serde::Serialize;
use std::path::{Path, PathBuf};

impl From<ArtifactError> for LauncherError {
    fn from(e: ArtifactError) -> Self {
        tracing::error!(error = %e, "error del almacen");
        match e {
            ArtifactError::InvalidHash | ArtifactError::Mismatch => {
                Self::InvalidInput("El archivo no coincide con lo esperado.".into())
            }
            _ => Self::Storage,
        }
    }
}

/// Resultado de poner al dia una instancia, pensado para mostrarse al usuario.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdoptSummary {
    pub adopted: usize,
    pub saved_bytes: u64,
}

impl Launcher {
    pub(crate) fn artifact_store(&self) -> Result<ArtifactStore, LauncherError> {
        Ok(ArtifactStore::open(self.paths().store_dir())?)
    }

    /// Adopta los archivos de `game_dir` sin cortar la operacion que lo pidio: es una optimizacion
    /// de disco, asi que un fallo solo se registra.
    pub(crate) async fn adopt_game_dir(&self, instance_id: &str, game_dir: &Path) {
        if let Err(e) = self.adopt_dir(instance_id, game_dir.to_path_buf()).await {
            tracing::warn!(instance = %instance_id, error = %e, "no se pudo compartir los archivos de la instancia");
        }
    }

    async fn adopt_dir(&self, instance_id: &str, game_dir: PathBuf) -> Result<AdoptReport, LauncherError> {
        let store = self.artifact_store()?;
        let events = std::sync::Arc::clone(&self.events);
        let id = instance_id.to_string();
        tokio::task::spawn_blocking(move || {
            adopt_instance(&store, &game_dir, &|completed, total| {
                events.install_progress(InstallProgressEvent { instance_id: id.clone(), phase: "adopt", completed, total });
            })
        })
        .await
        .map_err(|_| LauncherError::Storage)?
        .map_err(Into::into)
    }

    /// Pone al dia el almacen con los mods de una instancia (la reserva, igual que el resto de cambios de mods).
    pub async fn store_adopt_instance(&self, instance_id: &str) -> Result<AdoptSummary, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let instance = self.instances.get(instance_id).await?.ok_or(LauncherError::InstanceNotFound)?;
        let report = self.adopt_dir(instance_id, instance.game_dir).await?;
        Ok(AdoptSummary { adopted: report.adopted, saved_bytes: report.saved_bytes })
    }
}
