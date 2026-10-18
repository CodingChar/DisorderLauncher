//! Importar y exportar modpacks `.mrpack`. La importacion es transaccional: si algo falla,
//! la instancia creada y lo descargado hasta ese momento se eliminan (rollback).

use crate::{InstallProgressEvent, Launcher, LauncherError};
use instance_manager::{Instance, InstanceRepository, LoaderKind, NewInstance, MAX_NAME_LEN};
use modpacks::{ExportInput, ModpackError};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub remote_files: usize,
    pub bundled_files: usize,
}

impl From<ModpackError> for LauncherError {
    fn from(e: ModpackError) -> Self {
        tracing::error!(error = %e, "error de modpack");
        match e {
            ModpackError::NotAModpack | ModpackError::UnsupportedFormat(_) => {
                Self::InvalidInput("Ese archivo no es un modpack valido (.mrpack).".into())
            }
            ModpackError::OtherLauncher(name) => Self::InvalidInput(format!(
                "Ese archivo es un modpack de {name}. Este launcher usa el formato .mrpack de Modrinth: busca el mismo modpack en Modrinth o exporta uno en ese formato."
            )),
            ModpackError::UnsupportedLoader(name) => {
                Self::InvalidInput(format!("Este modpack necesita {name}, que aun no esta disponible."))
            }
            ModpackError::UnsafePath(_) | ModpackError::UntrustedSource(_) | ModpackError::MissingHash(_) | ModpackError::TooLarge => {
                Self::InvalidInput("Este modpack no es seguro y no se puede instalar.".into())
            }
            ModpackError::DownloadFailures(_) | ModpackError::Download(_) => Self::Offline,
            ModpackError::Http(h) if crate::error::is_network(&h) => Self::Offline,
            ModpackError::Archive(_) => Self::InvalidInput("El modpack esta danado o no se puede abrir.".into()),
            _ => Self::Storage,
        }
    }
}

/// Nombre valido para una instancia a partir del nombre del modpack.
fn instance_name(pack_name: &str) -> String {
    let cleaned: String = pack_name.chars().filter(|c| !c.is_control()).take(MAX_NAME_LEN).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "Modpack importado".to_string() } else { trimmed.to_string() }
}

impl Launcher {
    /// Crea una instancia a partir de un `.mrpack`: descarga sus archivos y aplica los overrides.
    pub async fn modpack_import(&self, mrpack: &Path) -> Result<Instance, LauncherError> {
        let profile_id = self.accounts_default().await?.ok_or(LauncherError::NoDefaultProfile)?;

        let path = mrpack.to_path_buf();
        let index = tokio::task::spawn_blocking(move || modpacks::read_index(&path))
            .await
            .map_err(|_| LauncherError::Storage)??;

        let (loader, loader_version) = match index.loader()? {
            Some(required) => {
                self.loader_provider(required.kind)?;
                (required.kind, Some(required.version))
            }
            None => (LoaderKind::Vanilla, None),
        };

        let instance = self
            .instances
            .create(NewInstance {
                name: instance_name(&index.name),
                minecraft_version: index.minecraft_version()?.to_string(),
                loader,
                loader_version,
                profile_id,
            })
            .await?;

        // La instancia queda reservada mientras se importa: no se puede lanzar ni editar.
        self.running_guard().insert(instance.id.clone(), None);
        let result = self.fill_instance_from_pack(&instance, &index, mrpack).await;
        self.running_guard().remove(&instance.id);

        match result {
            Ok(()) => Ok(instance),
            Err(e) => {
                if let Err(cleanup) = self.instances.delete(&instance.id).await {
                    tracing::warn!(error = %cleanup, "no se pudo limpiar la instancia tras un modpack fallido");
                }
                Err(e)
            }
        }
    }

    async fn fill_instance_from_pack(
        &self,
        instance: &Instance,
        index: &modpacks::ModpackIndex,
        mrpack: &Path,
    ) -> Result<(), LauncherError> {
        let events = Arc::clone(&self.events);
        let id = instance.id.clone();
        modpacks::download_pack_files(
            &self.http,
            index,
            &instance.game_dir,
            self.settings().download_profile.concurrency(),
            Arc::new(move |completed, total| {
                events.install_progress(InstallProgressEvent {
                    instance_id: id.clone(),
                    phase: "modpack",
                    completed,
                    total,
                })
            }),
        )
        .await?;
        modpacks::apply_overrides(mrpack, &instance.game_dir).await?;
        self.register_pack_mods(instance, index).await;
        Ok(())
    }

    /// Registra los mods del modpack como mods de Modrinth (por su hash) para que se puedan actualizar.
    /// Es opcional: sin red o si Modrinth no responde, los mods quedan como locales y todo sigue igual.
    async fn register_pack_mods(&self, instance: &Instance, index: &modpacks::ModpackIndex) {
        let files: Vec<(String, String)> = index
            .client_files()
            .filter_map(|f| {
                let name = f.path.strip_prefix("mods/")?;
                (!name.contains('/') && name.ends_with(".jar")).then(|| (name.to_string(), f.hashes.sha1.clone()))
            })
            .collect();
        if files.is_empty() {
            return;
        }
        let manager = mods::ModManager::new(&instance.game_dir);
        let registered = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            self.mods_service.register_known(&manager, &files),
        )
        .await;
        match registered {
            Ok(Ok(count)) => tracing::info!(count, "mods del modpack registrados como mods de Modrinth"),
            _ => tracing::warn!("no se pudieron registrar los mods del modpack; quedan como locales"),
        }
    }

    /// Exporta una instancia como `.mrpack`. No incluye mundos, logs ni datos de cuentas.
    pub async fn modpack_export(&self, instance_id: &str, dest: &Path) -> Result<ExportSummary, LauncherError> {
        self.ensure_idle(instance_id)?;
        let instance = self
            .instances
            .get(instance_id)
            .await?
            .ok_or(LauncherError::InstanceNotFound)?;

        let loader = instance
            .loader_version
            .as_deref()
            .map(|version| (instance.loader, version))
            .filter(|(kind, _)| *kind != LoaderKind::Vanilla);
        // Una version personalizada (OptiFine...) se exporta con la version de Minecraft en la que se apoya.
        let minecraft_version = self.base_minecraft_version(&instance).await;
        let input = ExportInput {
            name: &instance.name,
            version_id: "1.0.0",
            summary: None,
            minecraft_version: &minecraft_version,
            loader,
            game_dir: &instance.game_dir,
        };
        let report = modpacks::export_mrpack(&self.http, &input, dest).await?;
        Ok(ExportSummary { remote_files: report.remote_files, bundled_files: report.bundled_files })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_names_are_sanitized() {
        assert_eq!(instance_name("  Mi pack  "), "Mi pack");
        assert_eq!(instance_name("a\nb"), "ab");
        assert_eq!(instance_name("\n\t"), "Modpack importado");
        assert_eq!(instance_name(&"x".repeat(200)).chars().count(), MAX_NAME_LEN);
    }
}
