//! Operaciones de mods por instancia. La logica vive en el crate `mods`; aqui solo se
//! resuelve la instancia, se protege contra usarla mientras corre y se traducen errores.

use crate::{Launcher, LauncherError};
use instance_manager::{Instance, InstanceRepository, LoaderKind};
use mods::{
    ContentKind, DependencyReport, InstalledMod, InstanceTarget, MissingDependency, ModConflict, ModEntry, ModError, ModManager, ProviderInfo,
    SearchPage, SearchQuery, Side, SortKey, UpdateInfo, CATEGORIES, MODRINTH,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Con que se compara lo que se busca. La instancia manda por defecto: lo demas es para curiosear o
/// para mods que se usaran con otra version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Compatibility {
    /// Version y loader de la instancia.
    #[default]
    Instance,
    /// La version de la instancia, con cualquier loader.
    AnyLoader,
    /// El loader de la instancia, con cualquier version.
    AnyVersion,
    /// Sin filtrar por compatibilidad.
    Any,
}

/// Peticion de busqueda de la interfaz: filtros neutros mas lo que depende de la instancia.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModSearchRequest {
    /// `None`: el proveedor predeterminado.
    pub provider: Option<String>,
    pub text: String,
    pub kind: ContentKind,
    pub categories: Vec<String>,
    pub side: Option<Side>,
    pub sort: SortKey,
    pub compatibility: Compatibility,
    pub offset: u32,
}

/// Lo que la interfaz necesita saber para dibujar los filtros.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogInfo {
    pub providers: Vec<ProviderInfo>,
    pub categories: Vec<&'static str>,
}

const SEARCH_PAGE: u32 = 20;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModsOverview {
    pub mods: Vec<ModEntry>,
    pub missing_dependencies: Vec<MissingDependency>,
    /// Mods activos que no se llevan bien entre si (pueden cerrar el juego).
    pub conflicts: Vec<ModConflict>,
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

/// Loaders cuyos mods sirven en la instancia: Quilt carga tambien los de Fabric.
fn compatible_loaders(kind: LoaderKind) -> Vec<String> {
    match kind {
        LoaderKind::Quilt => vec!["quilt".into(), "fabric".into()],
        other => vec![loader_name(other)],
    }
}

/// Traduce la peticion de la interfaz a una consulta neutra, segun la instancia.
fn search_query(request: ModSearchRequest, minecraft_version: &str, loader: LoaderKind) -> SearchQuery {
    let by_version = matches!(request.compatibility, Compatibility::Instance | Compatibility::AnyLoader);
    let by_loader = matches!(request.compatibility, Compatibility::Instance | Compatibility::AnyVersion);
    SearchQuery {
        text: request.text,
        kind: request.kind,
        minecraft_version: by_version.then(|| minecraft_version.to_string()),
        loaders: if by_loader { compatible_loaders(loader) } else { Vec::new() },
        categories: request.categories,
        side: request.side,
        sort: request.sort,
        offset: request.offset,
        limit: SEARCH_PAGE,
    }
}

pub(crate) fn supports_mods(kind: LoaderKind) -> bool {
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
            ModError::ProviderUnavailable(name) => Self::InvalidInput(format!("{name} todavia no esta disponible.")),
            ModError::InvalidFileName => Self::InvalidInput("Nombre de archivo no valido.".into()),
            ModError::NotFound => Self::InvalidInput("No se encontro ese mod.".into()),
            ModError::Io(io) => Self::from_io(&io),
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
        let conflicts = if supports_mods(instance.loader) { manager.conflicts(&instance.minecraft_version).await? } else { Vec::new() };
        Ok(ModsOverview {
            conflicts,
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
        let entry = manager.add_local(source).await?;
        self.adopt_game_dir(instance_id, &instance.game_dir).await;
        Ok(entry)
    }

    pub async fn mods_search(&self, instance_id: &str, request: ModSearchRequest) -> Result<SearchPage, LauncherError> {
        let (instance, _) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let provider = request.provider.clone();
        let query = search_query(request, &instance.minecraft_version, instance.loader);
        Ok(self.mods_service.search(provider.as_deref(), &query).await?)
    }

    /// Proveedores de busqueda y categorias disponibles (para dibujar los filtros).
    pub fn mods_catalog(&self) -> CatalogInfo {
        CatalogInfo { providers: self.mods_service.catalog().providers(), categories: CATEGORIES.to_vec() }
    }

    /// Instala un mod de Modrinth con sus dependencias obligatorias.
    pub async fn mods_install(&self, instance_id: &str, project_id: &str, provider: Option<&str>) -> Result<Vec<InstalledMod>, LauncherError> {
        self.mods_service.ensure_installable(provider.unwrap_or(MODRINTH))?;
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        let concurrency = self.settings().download_profile.concurrency();
        Ok(self.mods_service.install(&manager, project_id, target, concurrency).await?)
    }

    /// Instala de golpe lo que los mods de la instancia piden y no esta (p. ej. Fabric API). Lo que no se encuentra
    /// para esta version se devuelve aparte; nunca se instala nada a medias.
    pub async fn mods_install_missing(&self, instance_id: &str) -> Result<DependencyReport, LauncherError> {
        let _turn = self.mods_guard(instance_id).await;
        self.ensure_idle(instance_id)?;
        let (instance, manager) = self.mod_context(instance_id).await?;
        Self::require_mod_support(&instance)?;
        let loader = loader_name(instance.loader);
        let target = InstanceTarget { minecraft_version: &instance.minecraft_version, loader: &loader };
        let concurrency = self.settings().download_profile.concurrency();
        Ok(self.mods_service.install_missing(&manager, target, concurrency).await?)
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

    fn query(compatibility: Compatibility, loader: LoaderKind) -> SearchQuery {
        search_query(ModSearchRequest { compatibility, text: "sodium".into(), offset: 20, ..ModSearchRequest::default() }, "1.20.1", loader)
    }

    #[test]
    fn compatibility_decides_what_is_filtered_by_the_instance() {
        let q = query(Compatibility::Instance, LoaderKind::Forge);
        assert_eq!((q.minecraft_version.as_deref(), q.loaders.as_slice()), (Some("1.20.1"), ["forge".to_string()].as_slice()));
        let q = query(Compatibility::AnyLoader, LoaderKind::Forge);
        assert_eq!((q.minecraft_version.as_deref(), q.loaders.len()), (Some("1.20.1"), 0));
        let q = query(Compatibility::AnyVersion, LoaderKind::Forge);
        assert_eq!((q.minecraft_version.as_deref(), q.loaders.len()), (None, 1));
        let q = query(Compatibility::Any, LoaderKind::Forge);
        assert_eq!((q.minecraft_version, q.loaders.len()), (None, 0));
    }

    #[test]
    fn quilt_also_accepts_fabric_mods_and_paging_is_kept() {
        let q = query(Compatibility::Instance, LoaderKind::Quilt);
        assert_eq!(q.loaders, ["quilt", "fabric"]);
        assert_eq!((q.offset, q.limit, q.text.as_str()), (20, SEARCH_PAGE, "sodium"));
    }
}
