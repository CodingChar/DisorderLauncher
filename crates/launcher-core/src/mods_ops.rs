//! Operaciones de mods por instancia. La logica vive en el crate `mods`; aqui solo se
//! resuelve la instancia, se protege contra usarla mientras corre y se traducen errores.

use crate::{Launcher, LauncherError};
use instance_manager::{Instance, InstanceRepository, LoaderKind};
use mods::{
    InstalledMod, InstanceTarget, MissingDependency, ModEntry, ModError, ModManager, ProjectHit, UpdateInfo,
};
use serde::Serialize;
use std::path::Path;

const SEARCH_PAGE: u32 = 20;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModsOverview {
    pub mods: Vec<ModEntry>,
    pub missing_dependencies: Vec<MissingDependency>,
    /// `false` si el tipo de instancia no admite mods (p. ej. Vanilla).
    pub supported: bool,
}

/// Categoria de Modrinth con la que se filtran mods. Quilt carga los mods de Fabric; Forge y NeoForge tienen la suya.
fn loader_name(kind: LoaderKind) -> String {
    match kind {
        LoaderKind::Quilt => "fabric".to_string(),
        other => format!("{other:?}").to_lowercase(),
    }
}

fn supports_mods(kind: LoaderKind) -> bool {
    !matches!(kind, LoaderKind::Vanilla)
}

impl From<ModError> for LauncherError {
    fn from(e: ModError) -> Self {
        tracing::error!(error = %e, "error de mods");
        match e {
            ModError::Http(h) if crate::error::is_network(&h) => Self::Offline,
            ModError::Download(download_manager::DownloadError::Http(h)) if crate::error::is_network(&h) => Self::Offline,
            ModError::NoFile | ModError::Download(_) => Self::Offline,
            ModError::NotCompatible(_) | ModError::MissingDependency(_) | ModError::NothingToRollback => {
                Self::InvalidInput(sentence(&e.to_string()))
            }
            ModError::NotAMod => Self::InvalidInput("Ese archivo no es un mod valido (Fabric, Quilt, Forge o NeoForge).".into()),
            ModError::InvalidFileName => Self::InvalidInput("Nombre de archivo no valido.".into()),
            ModError::NotFound => Self::InvalidInput("No se encontro ese mod.".into()),
            _ => Self::Storage,
        }
    }
}

/// Primera letra en mayuscula y punto final, para mostrarlo tal cual.
fn sentence(message: &str) -> String {
    let mut chars = message.chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    let text = format!("{first}{}", chars.as_str());
    if text.ends_with('.') { text } else { format!("{text}.") }
}

impl Launcher {
    async fn mod_context(&self, instance_id: &str) -> Result<(Instance, ModManager), LauncherError> {
        let instance = self
            .instances
            .get(instance_id)
            .await?
            .ok_or(LauncherError::InstanceNotFound)?;
        let manager = ModManager::new(&instance.game_dir);
        Ok((instance, manager))
    }

    /// Los mods solo se gestionan en instancias con loader compatible.
    fn require_mod_support(instance: &Instance) -> Result<(), LauncherError> {
        if supports_mods(instance.loader) {
            Ok(())
        } else {
            Err(LauncherError::InvalidInput("Esta instancia no admite mods. Crea una con Fabric, Quilt, Forge o NeoForge.".into()))
        }
    }

    pub async fn mods_list(&self, instance_id: &str) -> Result<ModsOverview, LauncherError> {
        let (instance, manager) = self.mod_context(instance_id).await?;
        let mods = manager.list().await?;
        Ok(ModsOverview {
            missing_dependencies: ModManager::missing_dependencies(&mods),
            mods,
            supported: supports_mods(instance.loader),
        })
    }

    pub async fn mods_set_enabled(&self, instance_id: &str, file: &str, enabled: bool) -> Result<(), LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (_, manager) = self.mod_context(instance_id).await?;
        manager.set_enabled(file, enabled).await?;
        Ok(())
    }

    pub async fn mods_remove(&self, instance_id: &str, file: &str) -> Result<(), LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (_, manager) = self.mod_context(instance_id).await?;
        Ok(manager.remove(file).await?)
    }

    /// Agrega un `.jar` elegido por el usuario en el selector de archivos.
    pub async fn mods_add_local(&self, instance_id: &str, source: &Path) -> Result<ModEntry, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        Ok(manager.add_local(source).await?)
    }

    pub async fn mods_search(&self, instance_id: &str, query: &str, page: u32) -> Result<Vec<ProjectHit>, LauncherError> {
        let (instance, _) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        Ok(self.mods_service.search(query, target, SEARCH_PAGE, page * SEARCH_PAGE).await?)
    }

    /// Instala un mod de Modrinth con sus dependencias obligatorias.
    pub async fn mods_install(&self, instance_id: &str, project_id: &str) -> Result<Vec<InstalledMod>, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        let concurrency = self.settings().download_profile.concurrency();
        Ok(self.mods_service.install(&manager, project_id, target, concurrency).await?)
    }

    pub async fn mods_check_updates(&self, instance_id: &str) -> Result<Vec<UpdateInfo>, LauncherError> {
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        Ok(self.mods_service.check_updates(&manager, target).await?)
    }

    pub async fn mods_update(&self, instance_id: &str, file: &str) -> Result<InstalledMod, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        Ok(self.mods_service.update(&manager, file, target).await?)
    }

    pub async fn mods_rollback(&self, instance_id: &str, file: &str) -> Result<(), LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (_, manager) = self.mod_context(instance_id).await?;
        Ok(self.mods_service.rollback(&manager, file).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_names_match_modrinth_categories() {
        assert_eq!(loader_name(LoaderKind::Fabric), "fabric");
        assert_eq!(loader_name(LoaderKind::NeoForge), "neoforge");
        assert_eq!(loader_name(LoaderKind::Quilt), "fabric");
    }

    #[test]
    fn messages_are_shown_as_sentences() {
        assert_eq!(sentence("Necesita Fabric API, que no esta disponible"), "Necesita Fabric API, que no esta disponible.");
        assert_eq!(sentence("sodium no es compatible."), "Sodium no es compatible.");
    }
}
