//! Exportar e importar una instancia como `.7z` portable (`dpack`). La importacion es transaccional,
//! igual que la de `.mrpack`: si algo falla, la instancia creada se elimina.

use crate::modpack_ops::GIB;
use crate::{InstallProgressEvent, Launcher, LauncherError};
use artifact_store::{is_shareable, mtime_ns, ArtifactEntry, ArtifactIndex, ArtifactSource, ArtifactStore, STORE_DIRS};
use dpack::{DpackError, EntrySource, ExtractPlan, Limits, Manifest, ManifestEntry};
use instance_manager::{Instance, InstanceRepository, LoaderKind, NewInstance, MAX_NAME_LEN};
use serde::Serialize;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DpackExportSummary {
    pub files: usize,
    pub bytes: u64,
}

impl From<DpackError> for LauncherError {
    fn from(e: DpackError) -> Self {
        tracing::error!(error = %e, "error de paquete de instancia");
        let invalid = |m: &str| Self::InvalidInput(m.to_string());
        match e {
            DpackError::NotADpack => invalid("Ese archivo no es un paquete de instancia (.7z de DisorderLauncher)."),
            DpackError::UnsupportedVersion(_) => {
                invalid("Este paquete se creo con una version mas nueva del launcher. Actualiza el launcher para abrirlo.")
            }
            DpackError::UnsafePath(_) | DpackError::Link(_) | DpackError::Duplicate(_) | DpackError::Manifest(_) => {
                invalid("Este paquete no es seguro y no se puede importar.")
            }
            DpackError::TooLarge | DpackError::TooManyEntries => invalid("Este paquete es demasiado grande para importarlo."),
            DpackError::Mismatch(_) | DpackError::Missing(_) | DpackError::Archive(_) => {
                invalid("El paquete esta danado o incompleto.")
            }
            DpackError::Io(_) | DpackError::Store(_) => Self::Storage,
        }
    }
}

fn loader_kind_name(kind: LoaderKind) -> &'static str {
    match kind {
        LoaderKind::Vanilla => "vanilla",
        LoaderKind::Fabric => "fabric",
        LoaderKind::Quilt => "quilt",
        LoaderKind::Forge => "forge",
        LoaderKind::NeoForge => "neoforge",
    }
}

fn parse_loader_kind(name: &str) -> Option<LoaderKind> {
    [LoaderKind::Vanilla, LoaderKind::Fabric, LoaderKind::Quilt, LoaderKind::Forge, LoaderKind::NeoForge]
        .into_iter()
        .find(|kind| loader_kind_name(*kind) == name)
}

fn pack_instance_name(pack_name: &str) -> String {
    let cleaned: String = pack_name.chars().filter(|c| !c.is_control()).take(MAX_NAME_LEN).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "Instancia importada".to_string() } else { trimmed.to_string() }
}

/// Archivos que viven en el almacen comun: jar/zip dentro de mods, resourcepacks o shaderpacks.
fn is_store_entry(path: &str) -> bool {
    // Solo archivos directamente dentro de la carpeta (igual que la adopcion): `mods/a.jar`, no `mods/x/a.jar`.
    let Some((dir, name)) = path.split_once('/') else { return false };
    STORE_DIRS.contains(&dir) && !name.contains('/') && is_shareable(name)
}

fn index_source(source: &EntrySource) -> ArtifactSource {
    match source {
        EntrySource::Local => ArtifactSource::Local,
        EntrySource::Modrinth { project_id, version_id } => {
            ArtifactSource::Modrinth { project_id: project_id.clone(), version_id: version_id.clone() }
        }
    }
}

impl Launcher {
    /// Exporta la instancia a un `.7z` autocontenido. No incluye cuentas, logs ni capturas; los mundos solo si se pide.
    pub async fn dpack_export(
        &self,
        instance_id: &str,
        dest: &Path,
        include_worlds: bool,
    ) -> Result<DpackExportSummary, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let instance = self.instances.get(instance_id).await?.ok_or(LauncherError::InstanceNotFound)?;
        // Con el contenido ya adoptado, el paquete sale del indice sin releer los mods.
        self.adopt_game_dir(instance_id, &instance.game_dir).await;

        let minecraft_version = self.base_minecraft_version(&instance).await;
        let dest = dest.to_path_buf();
        let events = Arc::clone(&self.events);
        let id = instance.id.clone();
        let report = tokio::task::spawn_blocking(move || {
            let loader_version = (instance.loader != LoaderKind::Vanilla).then_some(instance.loader_version.as_deref()).flatten();
            let request = dpack::ExportRequest {
                name: &instance.name,
                minecraft_version: &minecraft_version,
                loader_kind: loader_kind_name(instance.loader),
                loader_version,
                game_dir: &instance.game_dir,
                include_worlds,
            };
            dpack::export(&request, &dest, &|completed, total| {
                events.install_progress(InstallProgressEvent { instance_id: id.clone(), phase: "export7z", completed, total })
            })
        })
        .await
        .map_err(|_| LauncherError::Storage)??;
        Ok(DpackExportSummary { files: report.files, bytes: report.bytes })
    }

    /// Crea una instancia nueva a partir de un `.7z` exportado. Los mods que el almacen ya tiene no se vuelven a escribir.
    pub async fn dpack_import(&self, archive: &Path) -> Result<Instance, LauncherError> {
        let profile_id = self.accounts_default().await?.ok_or(LauncherError::NoDefaultProfile)?;

        let path = archive.to_path_buf();
        let manifest = tokio::task::spawn_blocking(move || dpack::read_manifest(&path, &Limits::default()))
            .await
            .map_err(|_| LauncherError::Storage)??;

        let loader = parse_loader_kind(&manifest.loader.kind)
            .ok_or_else(|| LauncherError::InvalidInput("Este paquete necesita un loader que no esta disponible.".into()))?;
        if loader != LoaderKind::Vanilla {
            self.loader_provider(loader)?;
            if manifest.loader.version.as_deref().is_none_or(str::is_empty) {
                return Err(DpackError::Manifest("falta la version del loader".into()).into());
            }
        }
        let loader_version = (loader != LoaderKind::Vanilla).then(|| manifest.loader.version.clone()).flatten();

        let minecraft_version = manifest.minecraft.version.clone();
        if matches!(self.resolve_descriptor(&minecraft_version).await, Err(LauncherError::VersionUnavailable)) {
            return Err(LauncherError::InvalidInput(format!(
                "Este paquete necesita Minecraft {minecraft_version}, que no esta disponible."
            )));
        }
        let total: u64 = manifest.entries.iter().map(|e| e.size).sum();
        self.ensure_free_space(total + total / 4 + GIB).await?;

        let instance = self
            .instances
            .create(NewInstance {
                name: pack_instance_name(&manifest.pack.name),
                minecraft_version,
                loader,
                loader_version,
                profile_id,
            })
            .await?;

        // Reservada mientras se importa; la marca en disco permite deshacerla si el launcher se cierra a medias.
        self.running_guard().insert(instance.id.clone(), None);
        let result = match crate::integrity::mark_import_pending(&instance.game_dir).await {
            Ok(()) => self.fill_instance_from_dpack(&instance, archive, manifest).await,
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
                    tracing::warn!(error = %cleanup, "no se pudo limpiar la instancia tras un paquete fallido");
                }
                Err(e)
            }
        }
    }

    async fn fill_instance_from_dpack(&self, instance: &Instance, archive: &Path, manifest: Manifest) -> Result<(), LauncherError> {
        let store = self.artifact_store()?;
        let events = Arc::clone(&self.events);
        let id = instance.id.clone();
        let game_dir = instance.game_dir.clone();
        let archive = archive.to_path_buf();

        let known_mods = tokio::task::spawn_blocking(move || {
            extract_into(&store, &archive, &manifest, &game_dir, &|completed, total| {
                events.install_progress(InstallProgressEvent { instance_id: id.clone(), phase: "import7z", completed, total })
            })
        })
        .await
        .map_err(|_| LauncherError::Storage)??;

        self.register_known_mods(instance, known_mods).await;
        Ok(())
    }
}

/// Extrae el paquete en `game_dir`. Lo compartible pasa por el almacen (y se omite si ya estaba); el
/// resto se escribe en su sitio. Devuelve (nombre, SHA-1) de los mods que vienen de Modrinth.
fn extract_into(
    store: &ArtifactStore,
    archive: &Path,
    manifest: &Manifest,
    game_dir: &Path,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<Vec<(String, String)>, DpackError> {
    let mut index = ArtifactIndex::default();
    let mut temps: Vec<PathBuf> = Vec::new();
    let mut counter = 0u32;

    let outcome = {
        let temps = &mut temps;
        let index = &mut index;
        let counter = &mut counter;
        dpack::extract(
            archive,
            manifest,
            &Limits::default(),
            &mut |entry: &ManifestEntry| {
                if !is_store_entry(&entry.path) {
                    return Ok(ExtractPlan::WriteTo(game_dir.join(&entry.path)));
                }
                if store.has(&entry.sha512) {
                    return Ok(ExtractPlan::Skip);
                }
                *counter += 1;
                let temp = store.tmp_dir().join(format!("imp-{}-{}.part", std::process::id(), counter));
                temps.push(temp.clone());
                Ok(ExtractPlan::WriteTo(temp))
            },
            &mut |entry: &ManifestEntry, written: Option<&Path>| {
                if !is_store_entry(&entry.path) {
                    return Ok(());
                }
                if let Some(temp) = written {
                    store.adopt_verified(temp, &entry.sha512)?;
                }
                let dest = game_dir.join(&entry.path);
                store.materialize(&entry.sha512, &dest)?;
                let mtime = std::fs::metadata(&dest).map(|m| mtime_ns(&m)).unwrap_or(0);
                index.entries.insert(
                    entry.path.clone(),
                    ArtifactEntry { sha512: entry.sha512.clone(), size: entry.size, mtime_ns: mtime, source: index_source(&entry.source) },
                );
                Ok(())
            },
            progress,
        )
    };
    for temp in &temps {
        let _ = std::fs::remove_file(temp);
    }
    outcome?;
    index.save(game_dir)?;

    let mut known = Vec::new();
    for entry in &manifest.entries {
        let name = entry.path.strip_prefix("mods/");
        if let (Some(name), EntrySource::Modrinth { .. }) = (name, &entry.source) {
            if !name.contains('/') && name.ends_with(".jar") {
                if let Ok(mut file) = std::fs::File::open(game_dir.join(&entry.path)) {
                    let mut hasher = Sha1::new();
                    if std::io::copy(&mut file, &mut hasher).is_ok() {
                        known.push((name.to_string(), format!("{:x}", hasher.finalize())));
                    }
                }
            }
        }
    }
    Ok(known)
}
