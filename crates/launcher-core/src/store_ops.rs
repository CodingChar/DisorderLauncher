//! Almacen comun de artefactos: incorpora los mods de cada instancia para que las copias
//! identicas compartan los mismos bytes en disco. La logica vive en el crate `artifact-store`.

use crate::{InstallProgressEvent, Launcher, LauncherError};
use artifact_store::{adopt_instance, AdoptReport, ArtifactError, ArtifactStore, DEFAULT_GRACE};
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
            ArtifactError::UnreadableIndex => Self::InvalidInput(
                "No se puede liberar espacio ahora: el registro de una instancia esta danado. Inicia esa instancia una vez e intentalo de nuevo.".into(),
            ),
            ArtifactError::Io(io) => Self::from_io(&io),
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

/// Uso del almacen comun, en lenguaje de personas: cuanto ocupa, cuanto se ahorra y cuanto se puede liberar.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageSummary {
    pub store_bytes: u64,
    pub shared_bytes: u64,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSummary {
    pub freed_bytes: u64,
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
            return;
        }
        self.sync_mod_sources(game_dir).await;
    }

    /// Pasa a los registros de la instancia el origen que ya conoce `mods.json` (proyecto y version de Modrinth),
    /// para que una exportacion conserve de donde vino cada mod. Un fallo solo se registra.
    pub(crate) async fn sync_mod_sources(&self, game_dir: &Path) {
        let lock = mods::load_lock(&mods::ModManager::new(game_dir).lock_file()).await;
        if lock.mods.is_empty() {
            return;
        }
        let game_dir = game_dir.to_path_buf();
        let result = tokio::task::spawn_blocking(move || -> Result<(), ArtifactError> {
            let mut index = artifact_store::ArtifactIndex::load(&game_dir);
            let mut changed = false;
            for locked in &lock.mods {
                for rel in [format!("mods/{}", locked.file), format!("mods/{}.disabled", locked.file)] {
                    let Some(entry) = index.entries.get(&rel) else { continue };
                    if entry.source == artifact_store::ArtifactSource::Local {
                        let source = artifact_store::ArtifactSource::Modrinth {
                            project_id: locked.project_id.clone(),
                            version_id: locked.version_id.clone(),
                        };
                        index.set_source(&rel, source);
                        changed = true;
                    }
                }
            }
            if changed {
                index.save(&game_dir)?;
            }
            Ok(())
        })
        .await;
        if !matches!(result, Ok(Ok(()))) {
            tracing::warn!("no se pudo anotar el origen de los mods");
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

    /// Reserva todas las instancias (en orden fijo) y comprueba que nada se este instalando ni jugando:
    /// ni las estadisticas ni la limpieza deben cruzarse con un cambio de mods.
    async fn store_exclusive(&self) -> Result<(Vec<PathBuf>, Vec<tokio::sync::OwnedMutexGuard<()>>), LauncherError> {
        let mut list = self.instances.list().await?;
        list.sort_by(|a, b| a.id.cmp(&b.id));
        let mut guards = Vec::with_capacity(list.len());
        for instance in &list {
            guards.push(self.mods_guard(&instance.id).await);
        }
        if !self.running_guard().is_empty() {
            return Err(LauncherError::InvalidInput(
                "Cierra el juego y espera a que terminen las instalaciones para liberar espacio.".into(),
            ));
        }
        Ok((list.into_iter().map(|i| i.game_dir).collect(), guards))
    }

    pub async fn store_summary(&self) -> Result<StorageSummary, LauncherError> {
        let (dirs, _guards) = self.store_exclusive().await?;
        let store = self.artifact_store()?;
        let stats = tokio::task::spawn_blocking(move || artifact_store::stats(&store, &dirs, DEFAULT_GRACE))
            .await
            .map_err(|_| LauncherError::Storage)??;
        Ok(StorageSummary { store_bytes: stats.store_bytes, shared_bytes: stats.shared_bytes, reclaimable_bytes: stats.orphan_bytes })
    }

    /// Libera los archivos del almacen que ya no usa ninguna instancia.
    pub async fn store_cleanup(&self) -> Result<CleanupSummary, LauncherError> {
        let (dirs, _guards) = self.store_exclusive().await?;
        let store = self.artifact_store()?;
        let report = tokio::task::spawn_blocking(move || artifact_store::gc(&store, &dirs, DEFAULT_GRACE))
            .await
            .map_err(|_| LauncherError::Storage)??;
        tracing::info!(removed = report.removed, freed = report.freed_bytes, "limpieza del almacen");
        Ok(CleanupSummary { freed_bytes: report.freed_bytes })
    }

    /// Pone al dia el almacen con los mods de una instancia (la reserva, igual que el resto de cambios de mods).
    pub async fn store_adopt_instance(&self, instance_id: &str) -> Result<AdoptSummary, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let instance = self.instances.get(instance_id).await?.ok_or(LauncherError::InstanceNotFound)?;
        let report = self.adopt_dir(instance_id, instance.game_dir.clone()).await?;
        self.sync_mod_sources(&instance.game_dir).await;
        Ok(AdoptSummary { adopted: report.adopted, saved_bytes: report.saved_bytes })
    }
}
