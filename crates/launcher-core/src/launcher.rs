use crate::settings::{self, Settings};
use crate::{
    GameOutputEvent, IdentityRegistry, InstallProgressEvent, LaunchState, LaunchStateEvent,
    LauncherError, LauncherEvents, LauncherPaths, OutputStream, RepairSummary,
    auto_memory, system_memory_mb,
};
use account_storage::{AccountRepository, FileAccountRepository};
use chrono::Utc;
use identity_api::{AccountRecord, IdentityProviderKind};
use identity_local::{new_local_account, LocalIdentityProvider};
use identity_microsoft::MicrosoftIdentityProvider;
use instance_manager::{
    FileInstanceRepository, Instance, InstanceRepository, InstanceUpdate, JavaSelection, LoaderKind,
    MemoryPolicy, NewInstance,
};
use java_runtime::{JavaInstallation, JavaManager, JavaRequirement};
use loaders::{FabricProvider, LoaderInstallContext, LoaderProvider, LoaderVersion, QuiltProvider};
use minecraft_install::{
    build_asset_artifacts, build_install_plan, legacy_assets_present, materialize_legacy_assets,
    InstallExecutor, InstallPhase, InstallPlan,
};
use minecraft_launch::{
    GameKiller, LaunchContext, LaunchEvent, LaunchPlan, LaunchPlanBuilder, MemorySettings,
    ProcessRunner,
};
use minecraft_meta::{AssetIndex, MetaError, VersionDescriptor, VersionEntry, VersionManifest, VersionType};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

const MANIFEST_TIMEOUT: Duration = Duration::from_secs(20);
const DEFAULT_JAVA_MAJOR: u32 = 8;

/// Orquestador de la aplicacion (capa Application de la spec 02): une repositorios,
/// identidad, instalacion, Java y lanzamiento. No conoce Tauri.
pub struct Launcher {
    paths: LauncherPaths,
    pub(crate) http: reqwest::Client,
    accounts: Arc<dyn AccountRepository>,
    pub(crate) instances: Arc<FileInstanceRepository>,
    pub(crate) mods_service: mods::ModsService,
    registry: IdentityRegistry,
    loader_providers: HashMap<LoaderKind, Arc<dyn LoaderProvider>>,
    java: JavaManager,
    pub(crate) events: Arc<dyn LauncherEvents>,
    manifest_url: String,
    settings: Mutex<Settings>,
    pub(crate) theme_store: themes::ThemeStore,
    /// `None` mientras se prepara; `Some` cuando el proceso ya existe.
    running: Mutex<HashMap<String, Option<GameKiller>>>,
}

impl Launcher {
    pub fn new(paths: LauncherPaths, events: Arc<dyn LauncherEvents>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("DisorderLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        let mut registry = IdentityRegistry::new();
        registry.register(Arc::new(LocalIdentityProvider));
        registry.register(Arc::new(MicrosoftIdentityProvider));

        let mut loader_providers: HashMap<LoaderKind, Arc<dyn LoaderProvider>> = HashMap::new();
        loader_providers.insert(LoaderKind::Fabric, Arc::new(FabricProvider::create(http.clone())));
        loader_providers.insert(LoaderKind::Quilt, Arc::new(QuiltProvider::create(http.clone())));

        Self {
            loader_providers,
            mods_service: mods::ModsService::new(http.clone()),
            accounts: Arc::new(FileAccountRepository::new(paths.accounts_dir())),
            instances: Arc::new(FileInstanceRepository::new(paths.instances_dir())),
            java: JavaManager::new(http.clone(), paths.runtimes_dir()),
            registry,
            http,
            events,
            manifest_url: minecraft_meta::VERSION_MANIFEST_URL.to_string(),
            settings: Mutex::new(Settings::default()),
            theme_store: themes::ThemeStore::new(paths.themes_dir()),
            running: Mutex::new(HashMap::new()),
            paths,
        }
    }

    /// Carga los ajustes guardados. Llamar una vez al iniciar la app.
    pub async fn load_settings(&self) {
        let loaded = settings::load_settings(&self.paths.settings_file()).await;
        *self.settings_guard() = loaded;
    }

    pub fn settings_snapshot(&self) -> crate::SettingsSnapshot {
        crate::SettingsSnapshot {
            settings: self.settings(),
            data_dir: self.paths.root().display().to_string(),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings_guard().clone()
    }

    pub async fn settings_update(&self, new: Settings) -> Result<Settings, LauncherError> {
        let validated = new.validated()?;
        if let Some(id) = &validated.theme_id {
            if !self.theme_store.list().await.iter().any(|e| &e.theme.id == id) {
                return Err(LauncherError::InvalidInput("Ese tema ya no esta disponible.".into()));
            }
        }
        settings::save_settings(&self.paths.settings_file(), &validated).await?;
        *self.settings_guard() = validated.clone();
        Ok(validated)
    }

    fn settings_guard(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.settings.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn with_manifest_url(mut self, url: impl Into<String>) -> Self {
        self.manifest_url = url.into();
        self
    }

    pub fn paths(&self) -> &LauncherPaths {
        &self.paths
    }

    // ---- Cuentas -------------------------------------------------------------

    pub async fn accounts_list(&self) -> Result<Vec<AccountRecord>, LauncherError> {
        Ok(self.accounts.list().await?)
    }

    pub async fn accounts_default(&self) -> Result<Option<String>, LauncherError> {
        Ok(self.accounts.get_default().await?)
    }

    /// Crea un perfil Offline Gaming. El primero pasa a ser el predeterminado.
    pub async fn accounts_create_local(&self, name: &str) -> Result<AccountRecord, LauncherError> {
        let record = new_local_account(name)?;

        let existing = self.accounts.list().await?;
        let taken = existing.iter().any(|a| {
            a.provider == IdentityProviderKind::Local && a.label.eq_ignore_ascii_case(&record.label)
        });
        if taken {
            return Err(LauncherError::DuplicateProfile);
        }

        self.accounts.save(&record).await?;
        if self.accounts.get_default().await?.is_none() {
            self.accounts.set_default(&record.id).await?;
        }
        Ok(record)
    }

    pub async fn accounts_set_default(&self, id: &str) -> Result<(), LauncherError> {
        match self.accounts.set_default(id).await {
            Err(account_storage::StorageError::NotFound(_)) => Err(LauncherError::ProfileMissing),
            other => Ok(other?),
        }
    }

    // ---- Instancias ----------------------------------------------------------

    pub async fn instances_list(&self) -> Result<Vec<Instance>, LauncherError> {
        Ok(self.instances.list().await?)
    }

    /// Crea una instancia asociada al perfil predeterminado. Para loaders sin version
    /// explicita se elige la ultima estable compatible con `version`.
    pub async fn instances_create(
        &self,
        name: &str,
        version: &str,
        loader: LoaderKind,
        loader_version: Option<String>,
    ) -> Result<Instance, LauncherError> {
        let profile_id = self
            .accounts
            .get_default()
            .await?
            .ok_or(LauncherError::NoDefaultProfile)?;

        let loader_version = match loader {
            LoaderKind::Vanilla => None,
            kind => match loader_version {
                Some(v) => Some(v),
                None => Some(self.latest_stable_loader(kind, version).await?.version),
            },
        };

        Ok(self
            .instances
            .create(NewInstance {
                name: name.to_string(),
                minecraft_version: version.to_string(),
                loader,
                loader_version,
                profile_id,
            })
            .await?)
    }

    pub(crate) fn loader_provider(&self, kind: LoaderKind) -> Result<&Arc<dyn LoaderProvider>, LauncherError> {
        self.loader_providers.get(&kind).ok_or(LauncherError::LoaderNotSupported)
    }

    async fn latest_stable_loader(&self, kind: LoaderKind, version: &str) -> Result<LoaderVersion, LauncherError> {
        let versions = self.loader_provider(kind)?.versions(version).await?;
        versions
            .iter()
            .find(|v| v.stable)
            .or_else(|| versions.first())
            .cloned()
            .ok_or_else(|| LauncherError::LoaderUnavailable(format!("{kind:?}")))
    }

    /// Versiones del loader para una version de Minecraft (la mas reciente primero).
    pub async fn loader_versions(&self, kind: LoaderKind, version: &str) -> Result<Vec<LoaderVersion>, LauncherError> {
        Ok(self.loader_provider(kind)?.versions(version).await?)
    }

    /// Loaders que este launcher sabe instalar.
    pub fn supported_loaders(&self) -> Vec<LoaderKind> {
        let mut kinds: Vec<LoaderKind> = std::iter::once(LoaderKind::Vanilla)
            .chain(self.loader_providers.keys().copied())
            .collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        kinds
    }

    /// Edita una instancia. Se rechaza mientras el juego corre o se prepara.
    pub async fn instances_update(
        &self,
        id: &str,
        changes: InstanceUpdate,
    ) -> Result<Instance, LauncherError> {
        self.ensure_idle(id)?;
        if let Some(profile_id) = &changes.profile_id {
            self.accounts
                .get(profile_id)
                .await?
                .ok_or(LauncherError::ProfileMissing)?;
        }
        if let Some(JavaSelection::Path { path }) = &changes.java {
            if !java_runtime::java_executable(path).is_file() {
                return Err(LauncherError::InvalidInput(
                    "No se encontro Java en esa carpeta.".into(),
                ));
            }
        }
        Ok(self.instances.update(id, changes).await?)
    }

    pub async fn instances_delete(&self, id: &str) -> Result<(), LauncherError> {
        self.ensure_idle(id)?;
        Ok(self.instances.delete(id).await?)
    }

    pub async fn instances_duplicate(&self, id: &str) -> Result<Instance, LauncherError> {
        self.ensure_idle(id)?;
        Ok(self.instances.duplicate(id).await?)
    }

    pub(crate) fn ensure_idle(&self, id: &str) -> Result<(), LauncherError> {
        if self.running_guard().contains_key(id) {
            Err(LauncherError::AlreadyRunning)
        } else {
            Ok(())
        }
    }

    // ---- Versiones -----------------------------------------------------------

    /// Lista versiones de Minecraft. Por defecto solo las estables (`release`);
    /// `include_all` agrega snapshots y versiones historicas.
    pub async fn list_versions(&self, include_all: bool) -> Result<Vec<VersionEntry>, LauncherError> {
        let manifest = self.load_manifest().await?;
        Ok(manifest
            .versions
            .into_iter()
            .filter(|v| include_all || v.version_type == VersionType::Release)
            .collect())
    }

    async fn load_manifest(&self) -> Result<VersionManifest, LauncherError> {
        let cache = self.paths.manifest_cache();
        let fetched = tokio::time::timeout(
            MANIFEST_TIMEOUT,
            minecraft_meta::fetch_manifest_from(&self.http, &self.manifest_url),
        )
        .await;

        match fetched {
            Ok(Ok(manifest)) => {
                if let Err(e) = minecraft_meta::save_manifest_cache(&manifest, &cache).await {
                    tracing::warn!(error = %e, "no se pudo guardar el cache del manifiesto");
                }
                Ok(manifest)
            }
            failure => {
                tracing::warn!("no se pudo descargar el manifiesto, se usa el cache local");
                match minecraft_meta::load_cached_manifest(&cache).await {
                    Ok(manifest) => Ok(manifest),
                    Err(_) => match failure {
                        Ok(Err(e)) => Err(e.into()),
                        _ => Err(LauncherError::Offline),
                    },
                }
            }
        }
    }

    /// Descriptor de la version: del disco si ya esta instalada, si no de Mojang
    /// (verificando su SHA-1 contra el manifiesto) y se persiste.
    async fn resolve_descriptor(&self, version_id: &str) -> Result<VersionDescriptor, LauncherError> {
        let path = self.paths.version_descriptor(version_id);
        if let Ok(cached) = minecraft_meta::load_cached_version_descriptor(&path).await {
            return Ok(cached);
        }

        let manifest = self.load_manifest().await?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == version_id)
            .ok_or(LauncherError::VersionUnavailable)?;
        if !entry.url.starts_with("https://") {
            return Err(LauncherError::VersionUnavailable);
        }

        let net = |e: reqwest::Error| LauncherError::from(MetaError::Http(e));
        let bytes = self
            .http
            .get(&entry.url)
            .send()
            .await
            .map_err(net)?
            .error_for_status()
            .map_err(net)?
            .bytes()
            .await
            .map_err(net)?;

        let digest = format!("{:x}", Sha1::digest(&bytes));
        if !digest.eq_ignore_ascii_case(&entry.sha1) {
            return Err(LauncherError::CorruptDownload);
        }
        let descriptor: VersionDescriptor = serde_json::from_slice(&bytes).map_err(MetaError::from)?;

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(MetaError::from)?;
        }
        let part = path.with_extension("json.part");
        tokio::fs::write(&part, &bytes).await.map_err(MetaError::from)?;
        tokio::fs::rename(&part, &path).await.map_err(MetaError::from)?;
        Ok(descriptor)
    }

    // ---- Lanzamiento ---------------------------------------------------------

    /// Inicia la instancia en segundo plano. Devuelve en cuanto se acepta la
    /// peticion; el avance y los errores llegan por `LauncherEvents`.
    pub async fn launch(self: &Arc<Self>, instance_id: &str) -> Result<(), LauncherError> {
        let instance = self
            .instances
            .get(instance_id)
            .await?
            .ok_or(LauncherError::InstanceNotFound)?;

        {
            let mut running = self.running_guard();
            if running.contains_key(&instance.id) {
                return Err(LauncherError::AlreadyRunning);
            }
            running.insert(instance.id.clone(), None);
        }

        let this = Arc::clone(self);
        tokio::spawn(async move {
            let id = instance.id.clone();
            if let Err(e) = this.run(&instance).await {
                this.emit_state(&id, LaunchState::Failed { message: e.to_string(), kind: e.kind() });
            }
            this.running_guard().remove(&id);
        });
        Ok(())
    }

    /// Verifica la instalacion de una instancia y redescarga lo que falte o este danado.
    /// No se puede reparar mientras el juego corre o se prepara.
    pub async fn repair_instance(&self, instance_id: &str) -> Result<RepairSummary, LauncherError> {
        let instance = self
            .instances
            .get(instance_id)
            .await?
            .ok_or(LauncherError::InstanceNotFound)?;

        {
            let mut running = self.running_guard();
            if running.contains_key(&instance.id) {
                return Err(LauncherError::AlreadyRunning);
            }
            running.insert(instance.id.clone(), None);
        }
        let result = self.repair_inner(&instance).await;
        self.running_guard().remove(&instance.id);
        result
    }

    async fn repair_inner(&self, instance: &Instance) -> Result<RepairSummary, LauncherError> {
        let descriptor = self.resolve_descriptor(&instance.minecraft_version).await?;
        let data_dir = self.paths.minecraft_dir();
        let mut plan = build_install_plan(&descriptor, &data_dir);

        let events = Arc::clone(&self.events);
        let id = instance.id.clone();
        let executor = InstallExecutor::new(self.http.clone(), self.settings().download_profile.concurrency());
        let report = diagnostics::repair_installation(&executor, &mut plan, &data_dir, move |p| {
            events.repair_progress(InstallProgressEvent {
                instance_id: id.clone(),
                phase: phase_name(p.phase),
                completed: p.completed,
                total: p.total,
            })
        })
        .await?;

        Ok(RepairSummary {
            found: report.found,
            fixed: report.fixed(),
            remaining: report.after.issues.len() + usize::from(report.after.assets_unchecked),
        })
    }

    /// Cierra el juego de una instancia en ejecucion.
    pub fn stop(&self, instance_id: &str) -> Result<(), LauncherError> {
        match self.running_guard().get(instance_id) {
            Some(Some(killer)) => {
                killer.kill();
                Ok(())
            }
            Some(None) => Err(LauncherError::NotYetRunning),
            None => Err(LauncherError::InstanceNotFound),
        }
    }

    async fn run(&self, instance: &Instance) -> Result<(), LauncherError> {
        let id = instance.id.as_str();
        self.emit_state(id, LaunchState::Preparing);
        let plan = self.prepare_launch(instance).await?;

        self.emit_state(id, LaunchState::Starting);
        let mut game = ProcessRunner::launch(&plan).await?;
        self.running_guard().insert(instance.id.clone(), Some(game.killer()));

        let mut log = open_log(&self.instances.logs_dir(id)?).await;
        while let Some(event) = game.next_event().await {
            match event {
                LaunchEvent::Started { pid } => self.emit_state(id, LaunchState::Running { pid }),
                LaunchEvent::Stdout(line) => {
                    write_log(&mut log, &line).await;
                    self.events.game_output(
                        OutputStream::Stdout,
                        GameOutputEvent { instance_id: id.to_string(), line },
                    );
                }
                LaunchEvent::Stderr(line) => {
                    write_log(&mut log, &line).await;
                    self.events.game_output(
                        OutputStream::Stderr,
                        GameOutputEvent { instance_id: id.to_string(), line },
                    );
                }
                LaunchEvent::Exited { code } => self.emit_state(id, LaunchState::Exited { code }),
            }
        }
        Ok(())
    }

    /// Flujo de la spec 02: instancia -> perfil -> identidad -> version -> Java -> plan.
    pub async fn prepare_launch(&self, instance: &Instance) -> Result<LaunchPlan, LauncherError> {
        let id = instance.id.as_str();

        let mut account = self
            .accounts
            .get(&instance.profile_id)
            .await?
            .ok_or(LauncherError::ProfileMissing)?;
        // Un fallo de identidad se informa tal cual; nunca se degrada a perfil local.
        let provider = self
            .registry
            .get(account.provider)
            .ok_or_else(|| LauncherError::Identity("proveedor no disponible.".into()))?;
        let identity = provider.resolve_identity(&account).await?;

        let descriptor = self.resolve_descriptor(&instance.minecraft_version).await?;
        let data_dir = self.paths.minecraft_dir();

        let mut install_plan = build_install_plan(&descriptor, &data_dir);
        if !installation_complete(&install_plan, &data_dir).await {
            self.emit_state(id, LaunchState::Installing);
            self.install(id, &mut install_plan, &data_dir).await?;
        }

        let overlay = self.prepare_loader(instance, &data_dir).await?;
        let game_assets_dir = self.sync_instance_resources(instance, &install_plan, &data_dir).await?;

        self.emit_state(id, LaunchState::PreparingJava);
        let java = self.select_java(instance, &descriptor).await?;

        let mut ctx = LaunchContext::new(&data_dir, &instance.game_dir);
        ctx.game_assets_dir = game_assets_dir;
        ctx.overlay = overlay;
        ctx.memory = match &instance.memory {
            MemoryPolicy::Fixed { min_mb, max_mb } => {
                MemorySettings { min_mb: Some(*min_mb), max_mb: Some(*max_mb) }
            }
            MemoryPolicy::Auto => auto_memory(system_memory_mb()),
        };
        let plan = LaunchPlanBuilder::build(&descriptor, &identity, &java, &ctx)?;
        plan.validate()?;

        account.last_used_at = Some(Utc::now());
        if let Err(e) = self.accounts.save(&account).await {
            tracing::warn!(error = %e, "no se pudo actualizar el ultimo uso del perfil");
        }
        Ok(plan)
    }

    async fn install(
        &self,
        instance_id: &str,
        plan: &mut InstallPlan,
        data_dir: &Path,
    ) -> Result<(), LauncherError> {
        let events = Arc::clone(&self.events);
        let id = instance_id.to_string();
        let executor = InstallExecutor::new(self.http.clone(), self.settings().download_profile.concurrency());
        executor
            .execute(plan, data_dir, move |p| {
                events.install_progress(InstallProgressEvent {
                    instance_id: id.clone(),
                    phase: phase_name(p.phase),
                    completed: p.completed,
                    total: p.total,
                })
            })
            .await?;
        Ok(())
    }

    /// Instala (si hace falta) el loader de la instancia y devuelve su overlay de lanzamiento.
    /// Con la instalacion ya en disco no usa la red.
    async fn prepare_loader(
        &self,
        instance: &Instance,
        data_dir: &Path,
    ) -> Result<Option<minecraft_launch::LaunchOverlay>, LauncherError> {
        if instance.loader == LoaderKind::Vanilla {
            return Ok(None);
        }
        let provider = self.loader_provider(instance.loader)?;
        let loader_version = match &instance.loader_version {
            Some(v) => v.clone(),
            None => self.latest_stable_loader(instance.loader, &instance.minecraft_version).await?.version,
        };

        if let Some(installed) = provider
            .load_installed(data_dir, &instance.minecraft_version, &loader_version)
            .await?
        {
            return Ok(Some(installed.overlay));
        }

        self.emit_state(&instance.id, LaunchState::Installing);
        let events = Arc::clone(&self.events);
        let id = instance.id.clone();
        let ctx = LoaderInstallContext {
            data_dir,
            minecraft_version: &instance.minecraft_version,
            loader_version: &loader_version,
            concurrency: self.settings().download_profile.concurrency(),
            progress: Arc::new(move |completed, total| {
                events.install_progress(InstallProgressEvent {
                    instance_id: id.clone(),
                    phase: "loader",
                    completed,
                    total,
                })
            }),
        };
        Ok(Some(provider.install(&ctx).await?.overlay))
    }

    /// Versiones muy antiguas esperan sus assets en `<gameDir>/resources`: es por instancia.
    async fn sync_instance_resources(
        &self,
        instance: &Instance,
        plan: &InstallPlan,
        data_dir: &Path,
    ) -> Result<Option<PathBuf>, LauncherError> {
        let Ok(data) = tokio::fs::read_to_string(&plan.asset_index.path).await else {
            return Ok(None);
        };
        let Ok(index) = serde_json::from_str::<AssetIndex>(&data) else {
            return Ok(None);
        };
        if !index.map_to_resources {
            return Ok(None);
        }
        let index_id = plan
            .asset_index
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let resources = instance.game_dir.join("resources");
        materialize_legacy_assets(index, index_id, data_dir.to_path_buf(), Some(instance.game_dir.clone())).await?;
        Ok(Some(resources))
    }

    async fn select_java(
        &self,
        instance: &Instance,
        descriptor: &VersionDescriptor,
    ) -> Result<JavaInstallation, LauncherError> {
        match &instance.java {
            JavaSelection::Path { path } => Ok(JavaInstallation {
                path: path.clone(),
                version: "personalizado".into(),
                major: 0,
                managed: false,
            }),
            JavaSelection::Auto => {
                let major = descriptor
                    .java_version
                    .as_ref()
                    .map(|j| j.major_version)
                    .unwrap_or(DEFAULT_JAVA_MAJOR);
                if let Some(global) = self.global_java(major).await {
                    return Ok(global);
                }
                Ok(self.java.ensure_java(&JavaRequirement { major_version: major }).await?)
            }
        }
    }

    /// Java global de los ajustes, solo si es compatible con lo que pide el juego.
    async fn global_java(&self, required_major: u32) -> Option<JavaInstallation> {
        let path = self.settings().java_path?;
        let found = java_runtime::detect_java_at(&path).await.ok()?;
        let candidate = [found];
        let compatible = java_runtime::select_best_java(&candidate, required_major).is_some();
        if !compatible {
            tracing::warn!(required_major, "el Java global no es compatible; se usa el automatico");
        }
        let [found] = candidate;
        compatible.then_some(found)
    }

    fn emit_state(&self, instance_id: &str, state: LaunchState) {
        self.events.launch_state(LaunchStateEvent {
            instance_id: instance_id.to_string(),
            state,
        });
    }

    pub(crate) fn running_guard(&self) -> std::sync::MutexGuard<'_, HashMap<String, Option<GameKiller>>> {
        self.running.lock().unwrap_or_else(|p| p.into_inner())
    }
}

fn phase_name(phase: InstallPhase) -> &'static str {
    match phase {
        InstallPhase::Metadata => "metadata",
        InstallPhase::Client => "client",
        InstallPhase::Libraries => "libraries",
        InstallPhase::AssetIndex => "assetIndex",
        InstallPhase::Assets => "assets",
        InstallPhase::Natives => "natives",
        InstallPhase::Verify => "verify",
    }
}

/// `true` si todos los archivos del plan ya estan en disco, de modo que se pueda
/// jugar sin red. Solo comprueba existencia; la integridad es trabajo de repair.
pub async fn installation_complete(plan: &InstallPlan, data_dir: &Path) -> bool {
    let fixed = std::iter::once(&plan.client)
        .chain(std::iter::once(&plan.asset_index))
        .chain(plan.libraries.iter())
        .chain(plan.natives.iter());
    for artifact in fixed {
        if !is_file(&artifact.path).await {
            return false;
        }
    }

    if !plan.natives.is_empty() && !dir_has_entries(&plan.natives_dir).await {
        return false;
    }

    let Ok(data) = tokio::fs::read_to_string(&plan.asset_index.path).await else {
        return false;
    };
    let Ok(index) = serde_json::from_str::<AssetIndex>(&data) else {
        return false;
    };
    for asset in build_asset_artifacts(&index, data_dir) {
        if !is_file(&asset.path).await {
            return false;
        }
    }

    let index_id = plan
        .asset_index
        .path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    legacy_assets_present(&index, &index_id, data_dir, None)
}

async fn dir_has_entries(dir: &Path) -> bool {
    match tokio::fs::read_dir(dir).await {
        Ok(mut entries) => matches!(entries.next_entry().await, Ok(Some(_))),
        Err(_) => false,
    }
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path).await.map(|m| m.is_file()).unwrap_or(false)
}

async fn open_log(dir: &Path) -> Option<tokio::fs::File> {
    tokio::fs::create_dir_all(dir).await.ok()?;
    tokio::fs::File::create(dir.join("latest.log")).await.ok()
}

async fn write_log(log: &mut Option<tokio::fs::File>, line: &str) {
    if let Some(file) = log {
        let _ = file.write_all(line.as_bytes()).await;
        let _ = file.write_all(b"\n").await;
    }
}
