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

pub(crate) const GIB: u64 = 1024 * 1024 * 1024;

/// Espacio a reservar para importar: lo que pesan los archivos del modpack (con margen para overrides y
/// archivos temporales) mas lo que ocupa instalar Minecraft y su loader si aun no estan.
pub(crate) fn required_space_bytes(index: &modpacks::ModpackIndex) -> u64 {
    let files: u64 = index.client_files().map(|f| f.file_size).sum();
    files + files / 4 + GIB
}

/// Bytes libres en el disco que contiene `path` (el punto de montaje mas especifico). `None` si no se sabe.
pub(crate) fn free_space_bytes(path: &Path) -> Option<u64> {
    let probe = path.ancestors().find(|p| p.exists())?.canonicalize().ok()?;
    let plain = strip_verbatim(&probe);
    let kind = sysinfo::DiskRefreshKind::nothing().with_storage();
    let disks = sysinfo::Disks::new_with_refreshed_list_specifics(kind);
    disks
        .list()
        .iter()
        .filter(|d| plain.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space())
}

/// En Windows `canonicalize` devuelve rutas «verbatim» (con el prefijo `\\?\`): se quita para comparar con los discos.
fn strip_verbatim(path: &Path) -> std::path::PathBuf {
    let text = path.to_string_lossy();
    text.strip_prefix("\\\\?\\").map(std::path::PathBuf::from).unwrap_or_else(|| path.to_path_buf())
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

        // Fallar pronto y claro: sin version de Minecraft o sin espacio no tiene sentido empezar a descargar.
        let minecraft_version = index.minecraft_version()?.to_string();
        if matches!(self.resolve_descriptor(&minecraft_version).await, Err(LauncherError::VersionUnavailable)) {
            return Err(LauncherError::InvalidInput(format!(
                "Este modpack necesita Minecraft {minecraft_version}, que no esta disponible."
            )));
        }
        self.ensure_free_space(required_space_bytes(&index)).await?;

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

        // La instancia queda reservada mientras se importa: no se puede lanzar ni editar. La marca en disco
        // permite deshacer la importacion si el launcher se cierra a medias (`instances_integrity`).
        self.running_guard().insert(instance.id.clone(), None);
        let result = match crate::integrity::mark_import_pending(&instance.game_dir).await {
            Ok(()) => self.fill_instance_from_pack(&instance, &index, mrpack).await,
            Err(_) => Err(LauncherError::Storage),
        };
        if result.is_ok() {
            crate::integrity::clear_import_marker(&instance.game_dir).await;
        }
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

    /// Falla pronto y con un mensaje claro si el disco de las instancias no tiene `needed` bytes libres.
    pub(crate) async fn ensure_free_space(&self, needed: u64) -> Result<(), LauncherError> {
        // La enumeracion de discos es bloqueante (y lenta con unidades de red o opticas): fuera del runtime.
        let instances_dir = self.paths().instances_dir();
        let free = tokio::task::spawn_blocking(move || free_space_bytes(&instances_dir)).await.ok().flatten();
        match free {
            Some(free) if free < needed => Err(LauncherError::InvalidInput(format!(
                "No hay espacio suficiente en el disco: se necesitan unos {} GB libres y hay {}.",
                needed.div_ceil(GIB),
                free / GIB
            ))),
            _ => Ok(()),
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
        self.register_known_mods(instance, files).await;
    }

    /// `files` son (nombre en `mods/`, SHA-1). Es opcional: sin red, los mods quedan como locales.
    pub(crate) async fn register_known_mods(&self, instance: &Instance, files: Vec<(String, String)>) {
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
        let _turn = self.mods_guard(instance_id).await;
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
    fn space_needed_covers_the_files_plus_margin() {
        let json = r#"{"formatVersion":1,"game":"minecraft","versionId":"1","name":"p","files":[
            {"path":"mods/a.jar","hashes":{"sha1":"aa","sha512":"bb"},"downloads":["https://cdn.modrinth.com/a.jar"],"fileSize":400},
            {"path":"mods/b.jar","hashes":{"sha1":"aa","sha512":"bb"},"env":{"client":"unsupported","server":"required"},"downloads":["https://cdn.modrinth.com/b.jar"],"fileSize":9999}
        ],"dependencies":{"minecraft":"1.20.1"}}"#;
        let index = modpacks::parse_index(json.as_bytes()).unwrap();
        assert_eq!(required_space_bytes(&index), 400 + 100 + GIB, "los mods solo de servidor no cuentan");
    }

    #[test]
    fn free_space_of_a_real_folder_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let free = free_space_bytes(&dir.path().join("aun/no/existe")).expect("el disco de la carpeta existente");
        assert!(free > 0);
    }

    #[test]
    fn instance_names_are_sanitized() {
        assert_eq!(instance_name("  Mi pack  "), "Mi pack");
        assert_eq!(instance_name("a\nb"), "ab");
        assert_eq!(instance_name("\n\t"), "Modpack importado");
        assert_eq!(instance_name(&"x".repeat(200)).chars().count(), MAX_NAME_LEN);
    }
}
