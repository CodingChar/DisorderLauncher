use instance_manager::{InstanceRepository, JavaSelection, LoaderKind};
use launcher_core::{
    installation_complete, GameOutputEvent, InstallProgressEvent, LaunchState, LaunchStateEvent,
    Launcher, LauncherError, LauncherEvents, LauncherPaths, OutputStream,
};
use minecraft_install::build_install_plan;
use minecraft_meta::VersionDescriptor;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Puerto cerrado: cualquier descarga falla de inmediato, simulando falta de red.
const NO_NETWORK: &str = "http://127.0.0.1:1/version_manifest_v2.json";

#[derive(Default)]
struct Collector {
    states: Mutex<Vec<LaunchStateEvent>>,
    output: Mutex<Vec<String>>,
    progress: Mutex<Vec<InstallProgressEvent>>,
}

impl LauncherEvents for Collector {
    fn install_progress(&self, event: InstallProgressEvent) {
        self.progress.lock().unwrap().push(event);
    }
    fn launch_state(&self, event: LaunchStateEvent) {
        self.states.lock().unwrap().push(event);
    }
    fn game_output(&self, _: OutputStream, event: GameOutputEvent) {
        self.output.lock().unwrap().push(event.line);
    }
}

fn launcher() -> (tempfile::TempDir, Arc<Launcher>, Arc<Collector>) {
    let dir = tempfile::tempdir().unwrap();
    let events = Arc::new(Collector::default());
    let launcher = Launcher::new(LauncherPaths::new(dir.path()), events.clone())
        .with_manifest_url(NO_NETWORK);
    (dir, Arc::new(launcher), events)
}

const VERSION_JSON: &str = r#"{
  "id": "1.99",
  "type": "release",
  "mainClass": "net.minecraft.client.main.Main",
  "downloads": { "client": { "sha1": "aa", "size": 1, "url": "https://example.com/client.jar" } },
  "assetIndex": { "id": "idx", "sha1": "bb", "size": 1, "totalSize": 1, "url": "https://example.com/idx.json" },
  "assets": "idx",
  "libraries": [],
  "arguments": { "game": ["--username", "${auth_player_name}", "--uuid", "${auth_uuid}"], "jvm": ["-cp", "${classpath}"] }
}"#;

/// Deja una version "instalada" en disco y devuelve la ruta de un Java falso.
fn fake_installation(launcher: &Launcher, root: &std::path::Path) -> std::path::PathBuf {
    let paths = launcher.paths();
    let descriptor_path = paths.version_descriptor("1.99");
    std::fs::create_dir_all(descriptor_path.parent().unwrap()).unwrap();
    std::fs::write(&descriptor_path, VERSION_JSON).unwrap();
    std::fs::write(descriptor_path.with_file_name("1.99.jar"), b"jar").unwrap();

    let indexes = paths.minecraft_dir().join("assets").join("indexes");
    std::fs::create_dir_all(&indexes).unwrap();
    std::fs::write(indexes.join("idx.json"), r#"{"objects":{}}"#).unwrap();

    let jre = root.join("fake-jre");
    std::fs::create_dir_all(jre.join("bin")).unwrap();
    std::fs::write(jre.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" }), b"").unwrap();
    jre
}

async fn wait_for_state(events: &Collector, pred: impl Fn(&LaunchState) -> bool) -> LaunchState {
    for _ in 0..100 {
        if let Some(found) = events.states.lock().unwrap().iter().find(|e| pred(&e.state)) {
            return found.state.clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("no llego el estado esperado: {:?}", events.states.lock().unwrap());
}

#[tokio::test]
async fn first_profile_becomes_default_and_names_are_unique() {
    let (_d, launcher, _) = launcher();

    let first = launcher.accounts_create_local("David").await.unwrap();
    let second = launcher.accounts_create_local("Alex").await.unwrap();
    assert_eq!(launcher.accounts_default().await.unwrap(), Some(first.id.clone()));
    assert_eq!(launcher.accounts_list().await.unwrap().len(), 2);

    assert!(matches!(
        launcher.accounts_create_local("david").await,
        Err(LauncherError::DuplicateProfile)
    ));
    assert!(matches!(
        launcher.accounts_create_local("no valido!").await,
        Err(LauncherError::InvalidInput(_))
    ));

    launcher.accounts_set_default(&second.id).await.unwrap();
    assert_eq!(launcher.accounts_default().await.unwrap(), Some(second.id));
    assert!(matches!(
        launcher.accounts_set_default("acc_local_nope").await,
        Err(LauncherError::ProfileMissing)
    ));
}

#[tokio::test]
async fn account_json_uses_camel_case_without_secrets() {
    let (_d, launcher, _) = launcher();
    let account = launcher.accounts_create_local("David").await.unwrap();
    let json = serde_json::to_value(&account).unwrap();

    assert_eq!(json["provider"], "local");
    assert_eq!(json["providerData"]["type"], "local");
    assert_eq!(json["providerData"]["displayName"], "David");
    assert!(json["providerData"]["localUuid"].is_string());
    assert!(!json.to_string().to_lowercase().contains("token"));
}

#[tokio::test]
async fn instance_requires_a_profile() {
    let (_d, launcher, _) = launcher();
    assert!(matches!(
        launcher.instances_create("Mi mundo", "1.21", LoaderKind::Vanilla, None).await,
        Err(LauncherError::NoDefaultProfile)
    ));

    let account = launcher.accounts_create_local("David").await.unwrap();
    let instance = launcher.instances_create("Mi mundo", "1.21", LoaderKind::Vanilla, None).await.unwrap();
    assert_eq!(instance.profile_id, account.id);
    assert_eq!(launcher.instances_list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn versions_without_network_or_cache_ask_for_connection() {
    let (_d, launcher, _) = launcher();
    assert!(matches!(launcher.list_versions(false).await, Err(LauncherError::Offline)));
}

#[tokio::test]
async fn versions_fall_back_to_cached_manifest_and_filter_snapshots() {
    let (_d, launcher, _) = launcher();
    let cache = launcher.paths().manifest_cache();
    std::fs::create_dir_all(cache.parent().unwrap()).unwrap();
    std::fs::write(
        &cache,
        r#"{"latest":{"release":"1.21","snapshot":"24w01a"},"versions":[
          {"id":"24w01a","type":"snapshot","url":"https://x/a","time":"t","releaseTime":"t","sha1":"a"},
          {"id":"1.21","type":"release","url":"https://x/b","time":"t","releaseTime":"t","sha1":"b"}]}"#,
    )
    .unwrap();

    let releases = launcher.list_versions(false).await.unwrap();
    assert_eq!(releases.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), ["1.21"]);
    assert_eq!(launcher.list_versions(true).await.unwrap().len(), 2);
}

#[tokio::test]
async fn installed_instance_prepares_launch_without_network() {
    let (dir, launcher, _) = launcher();
    let jre = fake_installation(&launcher, dir.path());

    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("Offline", "1.99", LoaderKind::Vanilla, None).await.unwrap();
    let updated = instance_manager::FileInstanceRepository::new(launcher.paths().instances_dir())
        .update(
            &instance.id,
            instance_manager::InstanceUpdate {
                java: Some(JavaSelection::Path { path: jre }),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let plan = launcher.prepare_launch(&updated).await.unwrap();
    let args = plan.command_args().unwrap();
    let pos = args.iter().position(|a| a == "--username").unwrap();
    assert_eq!(args[pos + 1], "Steve");
    assert_eq!(plan.working_directory, updated.game_dir);

    let account = launcher.accounts_list().await.unwrap().remove(0);
    assert!(account.last_used_at.is_some());
}

#[tokio::test]
async fn installation_complete_detects_missing_files() {
    let (dir, launcher, _) = launcher();
    let descriptor: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let data_dir = launcher.paths().minecraft_dir();
    let plan = build_install_plan(&descriptor, &data_dir);
    assert!(!installation_complete(&plan, &data_dir).await);

    fake_installation(&launcher, dir.path());
    assert!(installation_complete(&plan, &data_dir).await);

    std::fs::remove_file(&plan.client.path).unwrap();
    assert!(!installation_complete(&plan, &data_dir).await);
}

#[tokio::test]
async fn launch_reports_offline_failure_and_can_be_retried() {
    let (_d, launcher, events) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("Sin red", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    launcher.launch(&instance.id).await.unwrap();
    let state = wait_for_state(&events, |s| matches!(s, LaunchState::Failed { .. })).await;
    assert_eq!(
        state,
        LaunchState::Failed { message: LauncherError::Offline.to_string(), kind: launcher_core::ErrorKind::Offline }
    );

    // La instancia vuelve a quedar libre tras el fallo.
    let mut retried = false;
    for _ in 0..40 {
        match launcher.launch(&instance.id).await {
            Ok(()) => {
                retried = true;
                break;
            }
            Err(LauncherError::AlreadyRunning) => tokio::time::sleep(Duration::from_millis(25)).await,
            Err(e) => panic!("{e}"),
        }
    }
    assert!(retried);
}

#[tokio::test]
async fn launch_and_stop_of_unknown_instance_fail_cleanly() {
    let (_d, launcher, _) = launcher();
    assert!(matches!(launcher.launch("inst_nope").await, Err(LauncherError::InstanceNotFound)));
    assert!(launcher.stop("inst_nope").is_ok(), "detener algo que no corre es inofensivo");
    assert!(matches!(launcher.launch("../x").await, Err(LauncherError::InstanceNotFound)));
}

/// Prueba contra los servidores reales de Mojang. Ejecutar con `cargo test -- --ignored`.
#[tokio::test]
#[ignore = "requiere internet"]
async fn real_manifest_lists_releases_and_resolves_a_descriptor() {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default()));

    let releases = launcher.list_versions(false).await.unwrap();
    assert!(releases.iter().any(|v| v.id == "1.21"));
    assert!(launcher.paths().manifest_cache().is_file());
}

#[tokio::test]
async fn repair_of_unknown_instance_fails_cleanly() {
    let (_d, launcher, _) = launcher();
    assert!(matches!(launcher.repair_instance("inst_nope").await, Err(LauncherError::InstanceNotFound)));
}

/// Flujo real completo con una version pequena (1.0): descarga, natives, Java gestionado,
/// lanzamiento y cierre. Ejecutar con `cargo test -p launcher-core --test launcher -- --ignored --nocapture real_vanilla`.
#[tokio::test]
#[ignore = "requiere internet, descarga ~100 MB y abre una ventana de Minecraft unos segundos"]
async fn real_vanilla_install_launch_and_stop() {
    let dir = tempfile::tempdir().unwrap();
    let events = Arc::new(Collector::default());
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(dir.path()), events.clone()));

    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("E2E", "1.0", LoaderKind::Vanilla, None).await.unwrap();
    launcher.launch(&instance.id).await.unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(900);
    let running = loop {
        {
            let states = events.states.lock().unwrap();
            if let Some(e) = states.iter().find(|e| matches!(e.state, LaunchState::Running { .. } | LaunchState::Failed { .. } | LaunchState::Exited { .. })) {
                break e.state.clone();
            }
        }
        assert!(std::time::Instant::now() < deadline, "timeout esperando Running");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    println!("estados: {:?}", events.states.lock().unwrap().iter().map(|e| e.state.clone()).collect::<Vec<_>>());
    assert!(matches!(running, LaunchState::Running { .. }), "{running:?}");

    // Dejar que arranque unos segundos y cerrar.
    tokio::time::sleep(Duration::from_secs(8)).await;
    println!("salida del juego:
{}", events.output.lock().unwrap().join("
"));
    println!("estados finales: {:?}", events.states.lock().unwrap().iter().map(|e| e.state.clone()).collect::<Vec<_>>());
    launcher.stop(&instance.id).unwrap();
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;

    // Segundo arranque: todo esta instalado y el Java gestionado se reutiliza, sin descargas.
    let before = events.states.lock().unwrap().len();
    let started = std::time::Instant::now();
    launcher.launch(&instance.id).await.unwrap();
    for _ in 0..200 {
        let seen_running = events.states.lock().unwrap()[before..]
            .iter()
            .any(|e| matches!(e.state, LaunchState::Running { .. }));
        if seen_running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let second = events.states.lock().unwrap()[before..].iter().map(|e| e.state.clone()).collect::<Vec<_>>();
    println!("segundo arranque en {:?}: {second:?}", started.elapsed());
    assert!(!second.contains(&LaunchState::Installing), "no debia reinstalar");
    assert!(second.iter().any(|s| matches!(s, LaunchState::Running { .. })));
    let _ = launcher.stop(&instance.id);
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;

    let log = launcher.paths().instances_dir().join(&instance.id).join("logs").join("latest.log");
    println!("log existe: {}", log.is_file());
    assert!(log.is_file());
}

#[tokio::test]
async fn instance_crud_validates_java_and_ignores_per_instance_profile() {
    let (dir, launcher, _) = launcher();
    let first = launcher.accounts_create_local("David").await.unwrap();
    let second = launcher.accounts_create_local("Alex").await.unwrap();
    let instance = launcher.instances_create("Base", "1.21", LoaderKind::Vanilla, None).await.unwrap();
    assert_eq!(instance.profile_id, first.id);

    let updated = launcher
        .instances_update(
            &instance.id,
            instance_manager::InstanceUpdate {
                name: Some("Renombrada".into()),
                profile_id: Some(second.id.clone()),
                memory: Some(instance_manager::MemoryPolicy::Fixed { min_mb: 512, max_mb: 2048 }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    // El perfil no se elige por instancia: lo decide el menu de Perfiles y esa orden se ignora.
    assert_eq!((updated.name.as_str(), updated.profile_id.as_str()), ("Renombrada", first.id.as_str()));

    let bad_java = instance_manager::InstanceUpdate {
        java: Some(JavaSelection::Path { path: dir.path().join("no-java") }),
        ..Default::default()
    };
    assert!(matches!(launcher.instances_update(&instance.id, bad_java).await, Err(LauncherError::InvalidInput(_))));

    let jre = fake_installation(&launcher, dir.path());
    let good_java = instance_manager::InstanceUpdate { java: Some(JavaSelection::Path { path: jre }), ..Default::default() };
    launcher.instances_update(&instance.id, good_java).await.unwrap();

    let copy = launcher.instances_duplicate(&instance.id).await.unwrap();
    assert_eq!(copy.name, "Renombrada (copia)");
    assert_eq!(launcher.instances_list().await.unwrap().len(), 2);

    launcher.instances_delete(&copy.id).await.unwrap();
    assert_eq!(launcher.instances_list().await.unwrap().len(), 1);
    assert!(matches!(launcher.instances_delete("inst_nope").await, Ok(())));
}

#[tokio::test]
async fn busy_instance_cannot_be_edited_or_deleted() {
    let (_d, launcher, events) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("Ocupada", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    launcher.launch(&instance.id).await.unwrap();
    // Inmediatamente despues de aceptar el launch la instancia esta reservada.
    let blocked = launcher.instances_delete(&instance.id).await;
    let racing_done = events.states.lock().unwrap().iter().any(|e| matches!(e.state, LaunchState::Failed { .. }));
    if !racing_done {
        assert!(matches!(blocked, Err(LauncherError::AlreadyRunning)));
    }
}

#[tokio::test]
async fn settings_persist_across_launcher_restarts_and_validate_java() {
    use launcher_core::{DownloadProfile, Settings, Theme};
    let dir = tempfile::tempdir().unwrap();
    let make = || Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default()));

    let first = make();
    first.load_settings().await;
    assert_eq!(first.settings(), Settings::default());

    let updated = first
        .settings_update(Settings { theme: Theme::Light, download_profile: DownloadProfile::Performance, ..Settings::default() })
        .await
        .unwrap();
    assert_eq!(updated.download_profile.concurrency(), 10);

    let second = make();
    second.load_settings().await;
    assert_eq!(second.settings().theme, Theme::Light);

    let bad = Settings { java_path: Some(dir.path().join("nada")), ..Settings::default() };
    assert!(matches!(second.settings_update(bad).await, Err(LauncherError::InvalidInput(_))));
    assert_eq!(second.settings().theme, Theme::Light, "un fallo no altera los ajustes vigentes");
    assert_eq!(second.settings_snapshot().data_dir, dir.path().display().to_string());
}

/// Fabric real de punta a punta. Usa `DISORDER_E2E_DIR` (si existe) como raiz para reutilizar descargas:
/// `DISORDER_E2E_DIR=target/e2e-data cargo test -p launcher-core --test launcher -- --ignored --nocapture real_fabric`
#[tokio::test]
#[ignore = "requiere internet, descarga ~250 MB la primera vez y abre una ventana de Minecraft unos segundos"]
async fn real_fabric_install_launch_and_stop() {
    let temp;
    let root = match std::env::var("DISORDER_E2E_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => {
            temp = tempfile::tempdir().unwrap();
            temp.path().to_path_buf()
        }
    };
    let events = Arc::new(Collector::default());
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(&root), events.clone()));

    if launcher.accounts_list().await.unwrap().is_empty() {
        launcher.accounts_create_local("Steve").await.unwrap();
    }
    let instance = launcher
        .instances_create("E2E Fabric", "1.14.4", LoaderKind::Fabric, None)
        .await
        .unwrap();
    assert_eq!(instance.loader, LoaderKind::Fabric);
    assert!(instance.loader_version.is_some());

    launcher.launch(&instance.id).await.unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(1500);
    let first = loop {
        {
            let states = events.states.lock().unwrap();
            if let Some(e) = states.iter().find(|e| matches!(e.state, LaunchState::Running { .. } | LaunchState::Failed { .. } | LaunchState::Exited { .. })) {
                break e.state.clone();
            }
        }
        assert!(std::time::Instant::now() < deadline, "timeout");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    assert!(matches!(first, LaunchState::Running { .. }), "{first:?}");

    tokio::time::sleep(Duration::from_secs(15)).await;
    let output = events.output.lock().unwrap().join("\n");
    println!("salida del juego:\n{output}");
    let alive = events.states.lock().unwrap().iter().all(|e| !matches!(e.state, LaunchState::Exited { .. }));
    let _ = launcher.stop(&instance.id);
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;

    assert!(output.to_lowercase().contains("fabric"), "el juego debio cargar Fabric");
    assert!(alive, "el juego se cerro solo antes de tiempo");
}

fn write_mod(dir: &std::path::Path, file: &str, meta_json: &str) -> std::path::PathBuf {
    use std::io::Write;
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(file);
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    writer.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default()).unwrap();
    writer.write_all(meta_json.as_bytes()).unwrap();
    writer.finish().unwrap();
    path
}

#[tokio::test]
async fn local_mod_management_on_a_fabric_instance() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    // Con version de loader explicita no se consulta la red.
    let instance = launcher
        .instances_create("Con mods", "1.20.1", LoaderKind::Fabric, Some("0.16.9".into()))
        .await
        .unwrap();

    let jar = write_mod(
        &dir.path().join("descargas"),
        "alpha.jar",
        r#"{"id":"alpha","name":"Alpha","version":"1.0","depends":{"fabric-api":"*"}}"#,
    );
    // El archivo se elige en el selector del sistema; aqui se llama directamente al backend.
    let added = launcher.mods_add_local(&instance.id, &jar).await.unwrap();
    assert_eq!(added.name, "Alpha");

    let overview = launcher.mods_list(&instance.id).await.unwrap();
    assert!(overview.supported);
    assert_eq!(overview.mods.len(), 1);
    assert_eq!(overview.missing_dependencies[0].requires, "Fabric API");

    launcher.mods_set_enabled(&instance.id, "alpha.jar", false).await.unwrap();
    let overview = launcher.mods_list(&instance.id).await.unwrap();
    assert!(!overview.mods[0].enabled);
    assert!(overview.missing_dependencies.is_empty(), "un mod desactivado no pide dependencias");

    launcher.mods_remove(&instance.id, "alpha.jar.disabled").await.unwrap();
    assert!(launcher.mods_list(&instance.id).await.unwrap().mods.is_empty());

    assert!(matches!(
        launcher.mods_set_enabled(&instance.id, "../evil.jar", true).await,
        Err(LauncherError::InvalidInput(_))
    ));
}

#[tokio::test]
async fn vanilla_instances_do_not_support_mods() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("Vanilla", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    assert!(!launcher.mods_list(&instance.id).await.unwrap().supported);
    let jar = write_mod(&dir.path().join("d"), "a.jar", r#"{"id":"a"}"#);
    assert!(matches!(launcher.mods_add_local(&instance.id, &jar).await, Err(LauncherError::InvalidInput(_))));
    assert!(matches!(launcher.mods_search(&instance.id, launcher_core::ModSearchRequest::default()).await, Err(LauncherError::InvalidInput(_))));
    assert!(matches!(launcher.mods_install(&instance.id, "AANobbMI", None).await, Err(LauncherError::InvalidInput(_))));
}

#[tokio::test]
async fn loader_support_list_and_vanilla_needs_no_provider() {
    let (_d, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    // Con version de loader explicita no se consulta la red.
    let forge = launcher.instances_create("Forge", "1.20.1", LoaderKind::Forge, Some("47.3.0".into())).await.unwrap();
    assert_eq!((forge.loader, forge.loader_version.as_deref()), (LoaderKind::Forge, Some("47.3.0")));
    assert_eq!(launcher.supported_loaders(), vec![LoaderKind::Fabric, LoaderKind::Forge, LoaderKind::NeoForge, LoaderKind::Quilt, LoaderKind::Vanilla]);
}

fn write_mrpack(path: &std::path::Path, index_json: &str, entries: &[(&str, &[u8])]) {
    use std::io::Write;
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("modrinth.index.json", options).unwrap();
    writer.write_all(index_json.as_bytes()).unwrap();
    for (name, data) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();
}

fn pack_index(files: &str, deps: &str) -> String {
    format!(r#"{{"formatVersion":1,"game":"minecraft","versionId":"1.0.0","name":"Pack de prueba","files":[{files}],"dependencies":{{{deps}}}}}"#)
}

#[tokio::test]
async fn imports_a_modpack_into_a_new_fabric_instance() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let pack = dir.path().join("pack.mrpack");
    write_mrpack(
        &pack,
        &pack_index("", r#""minecraft":"1.20.1","fabric-loader":"0.15.0""#),
        &[("overrides/config/pack.toml", b"hola"), ("overrides/options.txt", b"o")],
    );

    let instance = launcher.modpack_import(&pack).await.unwrap();
    assert_eq!(instance.name, "Pack de prueba");
    assert_eq!(instance.minecraft_version, "1.20.1");
    assert_eq!(instance.loader, LoaderKind::Fabric);
    assert_eq!(instance.loader_version.as_deref(), Some("0.15.0"));
    assert_eq!(std::fs::read(instance.game_dir.join("config/pack.toml")).unwrap(), b"hola");
    assert_eq!(launcher.instances_list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn import_rejects_unsupported_or_unsafe_packs_without_leaving_instances() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();

    let evil = dir.path().join("evil.mrpack");
    let file = r#"{"path":"../../evil.jar","hashes":{"sha1":"aa","sha512":"bb"},"downloads":["https://cdn.modrinth.com/a.jar"],"fileSize":1}"#;
    write_mrpack(&evil, &pack_index(file, r#""minecraft":"1.20.1""#), &[]);
    assert!(matches!(launcher.modpack_import(&evil).await, Err(LauncherError::InvalidInput(_))));

    let not_a_pack = dir.path().join("x.mrpack");
    std::fs::write(&not_a_pack, "no es un zip").unwrap();
    assert!(matches!(launcher.modpack_import(&not_a_pack).await, Err(LauncherError::InvalidInput(_))));

    assert!(launcher.instances_list().await.unwrap().is_empty());
}

#[tokio::test]
async fn failed_download_rolls_back_the_created_instance() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let pack = dir.path().join("broken.mrpack");
    let file = r#"{"path":"mods/missing.jar","hashes":{"sha1":"da39a3ee5e6b4b0d3255bfef95601890afd80709","sha512":"bb"},"downloads":["https://cdn.modrinth.com/data/no-such-project/versions/none/missing.jar"],"fileSize":1}"#;
    write_mrpack(&pack, &pack_index(file, r#""minecraft":"1.20.1""#), &[("overrides/config/a.toml", b"a")]);

    assert!(launcher.modpack_import(&pack).await.is_err());
    assert!(launcher.instances_list().await.unwrap().is_empty(), "rollback: sin instancias a medias");
    let leftovers = std::fs::read_dir(launcher.paths().instances_dir()).map(|d| d.count()).unwrap_or(0);
    assert_eq!(leftovers, 0, "rollback: sin carpetas a medias");
}

#[tokio::test]
async fn exported_instance_can_be_imported_again() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let source = launcher
        .instances_create("Mi coleccion", "1.20.1", LoaderKind::Fabric, Some("0.15.0".into()))
        .await
        .unwrap();
    write_mod(&source.game_dir.join("mods"), "local.jar", r#"{"id":"local","name":"Local"}"#);
    std::fs::write(source.game_dir.join("config").join("a.toml"), "x").unwrap();
    std::fs::create_dir_all(source.game_dir.join("saves").join("w")).unwrap();
    std::fs::write(source.game_dir.join("saves").join("w").join("level.dat"), "mundo").unwrap();

    let pack = dir.path().join("export").join("pack.mrpack");
    let summary = launcher.modpack_export(&source.id, &pack).await.unwrap();
    assert_eq!(summary.remote_files, 0);
    assert!(summary.bundled_files >= 2);

    let copy = launcher.modpack_import(&pack).await.unwrap();
    assert_ne!(copy.id, source.id);
    assert_eq!(copy.loader_version.as_deref(), Some("0.15.0"));
    assert!(copy.game_dir.join("mods/local.jar").is_file());
    assert!(copy.game_dir.join("config/a.toml").is_file());
    assert!(!copy.game_dir.join("saves/w/level.dat").exists(), "los mundos no se exportan");
}

/// Instala Fabric API desde Modrinth, exporta (el mod debe ir referenciado por URL+hash) y reimporta.
/// `cargo test -p launcher-core --test launcher -- --ignored real_modpack`
#[tokio::test]
#[ignore = "requiere internet"]
async fn real_modpack_export_and_import_use_modrinth_references() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let source = launcher
        .instances_create("Real", "1.20.1", LoaderKind::Fabric, Some("0.15.0".into()))
        .await
        .unwrap();
    let installed = launcher.mods_install(&source.id, "P7dR8mSH", None).await.unwrap();
    assert!(installed.iter().any(|m| m.title.to_lowercase().contains("fabric api")));

    let pack = dir.path().join("real.mrpack");
    let summary = launcher.modpack_export(&source.id, &pack).await.unwrap();
    assert_eq!(summary.remote_files, 1, "Fabric API va por URL y hash, no dentro del paquete");

    let index = modpacks::read_index(&pack).unwrap();
    assert!(index.files[0].downloads[0].starts_with("https://cdn.modrinth.com/"));
    assert!(!index.files[0].hashes.sha512.is_empty());

    let copy = launcher.modpack_import(&pack).await.unwrap();
    let mods = launcher.mods_list(&copy.id).await.unwrap();
    assert_eq!(mods.mods.len(), 1);
    assert_eq!(mods.mods[0].name, "Fabric API");
}

#[tokio::test]
async fn custom_themes_can_be_selected_and_falling_back_when_removed() {
    use launcher_core::Settings;
    let (dir, launcher, _) = launcher();
    let theme = include_str!("../../../theme_examples/default-dark.theme.json")
        .replace("disorderlaucher.default.dark", "mi.tema")
        .replace("Default Dark", "Mi tema");
    let _ = dir;

    assert_eq!(launcher.themes_list().await.len(), 3);
    let imported = launcher.themes_import(theme.as_bytes()).await.unwrap();
    assert_eq!(imported.id, "mi.tema");
    assert!(matches!(launcher.themes_import(theme.as_bytes()).await, Err(LauncherError::InvalidInput(_))));
    assert!(matches!(launcher.themes_import(b"{}").await, Err(LauncherError::InvalidInput(_))));

    // Seleccionar un tema que no existe se rechaza; uno existente se guarda.
    let missing = Settings { theme_id: Some("fantasma".into()), ..Settings::default() };
    assert!(matches!(launcher.settings_update(missing).await, Err(LauncherError::InvalidInput(_))));
    let chosen = Settings { theme_id: Some("mi.tema".into()), ..Settings::default() };
    assert_eq!(launcher.settings_update(chosen).await.unwrap().theme_id.as_deref(), Some("mi.tema"));

    // Al quitar el tema en uso, los ajustes vuelven al tema incluido.
    launcher.themes_remove("mi.tema").await.unwrap();
    assert_eq!(launcher.settings().theme_id, None);
    assert!(matches!(launcher.themes_remove("disorderlaucher.default.dark").await, Err(LauncherError::InvalidInput(_))));
}

#[tokio::test]
async fn skins_link_to_local_profiles_and_unlink_when_removed() {
    use identity_api::ProviderAccountData;
    let (_d, launcher, _) = launcher();
    let account = launcher.accounts_create_local("David").await.unwrap();

    let png = skins::encode_rgba(&vec![200u8; 64 * 64 * 4], 64, 64).unwrap();
    let skin = launcher.skins_import(&png, "Mi skin", None).await.unwrap();
    assert_eq!(launcher.skins_list().await.len(), 1);
    assert!(launcher.skins_read(&skin.id).await.unwrap().starts_with("data:image/png;base64,"));

    // Invalida: no es PNG / dimensiones erroneas.
    assert!(matches!(launcher.skins_import(b"no png", "x", None).await, Err(LauncherError::InvalidInput(_))));
    let wrong = skins::encode_rgba(&vec![0u8; 32 * 32 * 4], 32, 32).unwrap();
    assert!(matches!(launcher.skins_import(&wrong, "x", None).await, Err(LauncherError::InvalidInput(_))));

    launcher.accounts_set_skin(&account.id, Some(&skin.id)).await.unwrap();
    let skin_of = |records: Vec<identity_api::AccountRecord>| match &records[0].provider_data {
        ProviderAccountData::Local(d) => d.skin_id.clone(),
        _ => unreachable!(),
    };
    assert_eq!(skin_of(launcher.accounts_list().await.unwrap()), Some(skin.id.clone()));
    // El registro de la cuenta guarda solo el id, nunca los pixeles.
    let record = std::fs::read_to_string(launcher.paths().accounts_dir().join(format!("{}.json", account.id))).unwrap();
    assert!(record.contains(&skin.id) && !record.contains("base64") && !record.contains("PNG"));

    assert!(matches!(launcher.accounts_set_skin(&account.id, Some("skin_00000000000000000000000000000000")).await, Err(LauncherError::InvalidInput(_))));

    launcher.skins_remove(&skin.id).await.unwrap();
    assert_eq!(skin_of(launcher.accounts_list().await.unwrap()), None, "borrar la skin la desvincula");
    assert!(launcher.skins_list().await.is_empty());
}

#[tokio::test]
async fn microsoft_profiles_do_not_take_local_skins() {
    use identity_api::{AccountRecord, MicrosoftAccountMetadata, ProviderAccountData};
    let (_d, launcher, _) = launcher();
    let accounts = account_storage::FileAccountRepository::new(launcher.paths().accounts_dir());
    let ms = AccountRecord {
        id: "acc_ms_x".into(),
        provider: identity_api::IdentityProviderKind::Microsoft,
        label: "Notch".into(),
        created_at: chrono::Utc::now(),
        last_used_at: None,
        provider_data: ProviderAccountData::Microsoft(MicrosoftAccountMetadata {
            minecraft_uuid: None, minecraft_name: None, secure_refresh_token_key: None, last_successful_auth: None,
        }),
    };
    account_storage::AccountRepository::save(&accounts, &ms).await.unwrap();

    let png = skins::encode_rgba(&vec![1u8; 64 * 64 * 4], 64, 64).unwrap();
    let skin = launcher.skins_import(&png, "S", None).await.unwrap();
    assert!(matches!(launcher.accounts_set_skin("acc_ms_x", Some(&skin.id)).await, Err(LauncherError::InvalidInput(_))));
    assert!(matches!(launcher.accounts_set_skin("acc_nope", None).await, Err(LauncherError::AccountNotFound)));
}

#[tokio::test]
async fn editor_save_creates_and_updates_skins_with_validation() {
    let (_d, launcher, _) = launcher();
    let png = skins::encode_rgba(&vec![90u8; 64 * 64 * 4], 64, 64).unwrap();

    let created = launcher.skins_save(None, "Dibujo", &png, skins::SkinModel::Slim).await.unwrap();
    assert_eq!(created.model, skins::SkinModel::Slim);

    let mut other = vec![10u8; 64 * 64 * 4];
    other[3] = 0;
    let edited_png = skins::encode_rgba(&other, 64, 64).unwrap();
    let updated = launcher.skins_save(Some(&created.id), "Dibujo v2", &edited_png, skins::SkinModel::Classic).await.unwrap();
    assert_eq!((updated.id.as_str(), updated.name.as_str(), updated.model), (created.id.as_str(), "Dibujo v2", skins::SkinModel::Classic));
    assert_eq!(launcher.skins_list().await.len(), 1);

    let before = launcher.skins_read(&created.id).await.unwrap();
    for invalid_name in ["  ", "nombre\nincorrecto", &"x".repeat(49)] {
        assert!(matches!(launcher.skins_save(Some(&created.id), invalid_name, &png, skins::SkinModel::Slim).await, Err(LauncherError::InvalidInput(_))));
        assert_eq!(launcher.skins_read(&created.id).await.unwrap(), before, "un nombre rechazado no sustituye los pixeles");
        let asset = launcher.skins_list().await.pop().unwrap();
        assert_eq!((asset.name.as_str(), asset.model), ("Dibujo v2", skins::SkinModel::Classic));
    }

    assert!(matches!(launcher.skins_save(None, "x", b"no es png", skins::SkinModel::Classic).await, Err(LauncherError::InvalidInput(_))));
    assert!(matches!(launcher.skins_save(Some("skin_00000000000000000000000000000000"), "x", &png, skins::SkinModel::Classic).await, Err(LauncherError::InvalidInput(_))));
}

#[tokio::test]
async fn startup_notices_report_quarantined_profiles_once_and_keep_working() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    std::fs::write(dir.path().join("accounts").join("acc_roto.json"), "{ no es json").unwrap();

    let notices = launcher.startup_notices().await;
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(notices[0].contains("danado"));
    assert!(launcher.startup_notices().await.is_empty(), "el aviso se entrega una sola vez");
    assert_eq!(launcher.accounts_list().await.unwrap().len(), 1);
}

/// Lanza una instancia con un loader real y comprueba que el juego lo carga. Reutiliza `DISORDER_E2E_DIR`.
async fn run_loader_e2e(kind: LoaderKind, mc: &str, expected_in_output: &[&str]) {
    run_loader_e2e_with_mods(kind, mc, expected_in_output, &[]).await
}

/// Igual que `run_loader_e2e`, instalando antes los mods de Modrinth indicados (ids de proyecto).
async fn run_loader_e2e_with_mods(kind: LoaderKind, mc: &str, expected_in_output: &[&str], mod_projects: &[&str]) {
    let temp;
    let root = match std::env::var("DISORDER_E2E_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => {
            temp = tempfile::tempdir().unwrap();
            temp.path().to_path_buf()
        }
    };
    let events = Arc::new(Collector::default());
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(&root), events.clone()));
    if launcher.accounts_list().await.unwrap().is_empty() {
        launcher.accounts_create_local("Steve").await.unwrap();
    }

    let versions = launcher.loader_versions(kind, mc).await.unwrap();
    println!("{kind:?} {mc}: {} versiones, la mas reciente {}", versions.len(), versions[0].version);
    let instance = launcher.instances_create(&format!("E2E {kind:?}"), mc, kind, None).await.unwrap();
    println!("version elegida: {:?}", instance.loader_version);
    for project in mod_projects {
        let installed = launcher.mods_install(&instance.id, project, None).await.unwrap();
        println!("mods instalados: {:?}", installed.iter().map(|m| (&m.title, &m.version_number)).collect::<Vec<_>>());
    }

    launcher.launch(&instance.id).await.unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2400);
    let first = loop {
        {
            let states = events.states.lock().unwrap();
            if let Some(e) = states.iter().find(|e| matches!(e.state, LaunchState::Running { .. } | LaunchState::Failed { .. } | LaunchState::Exited { .. })) {
                break e.state.clone();
            }
        }
        assert!(std::time::Instant::now() < deadline, "timeout");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    assert!(matches!(first, LaunchState::Running { .. }), "{first:?}");

    tokio::time::sleep(Duration::from_secs(25)).await;
    let output = events.output.lock().unwrap().join("\n");
    println!("salida del juego:\n{}", output.lines().take(60).collect::<Vec<_>>().join("\n"));
    let alive = events.states.lock().unwrap().iter().all(|e| !matches!(e.state, LaunchState::Exited { .. }));
    let _ = launcher.stop(&instance.id);
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;

    let lower = output.to_lowercase();
    for needle in expected_in_output {
        assert!(lower.contains(&needle.to_lowercase()), "la salida debia contener {needle:?}");
    }
    assert!(alive, "el juego se cerro solo antes de tiempo");
}

#[tokio::test]
#[ignore = "requiere internet, ejecuta el instalador oficial de Forge y abre Minecraft unos segundos"]
async fn real_forge_install_launch_and_stop() {
    run_loader_e2e(LoaderKind::Forge, "1.14.4", &["forge"]).await;
}

#[tokio::test]
#[ignore = "requiere internet (~600 MB la primera vez), ejecuta el instalador de NeoForge y abre Minecraft unos segundos"]
async fn real_neoforge_install_launch_and_stop() {
    run_loader_e2e(LoaderKind::NeoForge, "1.21.1", &["neoforge"]).await;
}

#[tokio::test]
#[ignore = "requiere internet y abre Minecraft unos segundos"]
async fn real_quilt_install_launch_and_stop() {
    run_loader_e2e(LoaderKind::Quilt, "1.14.4", &["quilt"]).await;
}

/// Fabric + Fabric API instalada desde Modrinth: el juego debe cargar el mod.
#[tokio::test]
#[ignore = "requiere internet y abre Minecraft unos segundos"]
async fn real_fabric_with_modrinth_mod_loads_the_mod() {
    run_loader_e2e_with_mods(LoaderKind::Fabric, "1.14.4", &["fabric", "fabric-api"], &["P7dR8mSH"]).await;
}

/// Sin red y sin cache, el catalogo ofrece solo lo ya instalado y las instancias se crean con ello.
#[tokio::test]
async fn version_catalog_without_network_falls_back_to_installed_versions() {
    let dir = tempfile::tempdir().unwrap();
    let versions = dir.path().join("minecraft").join("versions");
    for (id, json) in [
        ("1.12.2", r#"{"id":"1.12.2","type":"release","releaseTime":"2017-09-18T08:39:46+00:00"}"#),
        ("fabric-loader-0.15.0-1.12.2", r#"{"id":"fabric-loader-0.15.0-1.12.2","inheritsFrom":"1.12.2"}"#),
    ] {
        std::fs::create_dir_all(versions.join(id)).unwrap();
        std::fs::write(versions.join(id).join(format!("{id}.json")), json).unwrap();
    }
    let launcher = Arc::new(
        Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default())).with_manifest_url("http://127.0.0.1:9/manifest.json"),
    );

    let vanilla = launcher.version_catalog(LoaderKind::Vanilla).await.unwrap();
    assert_eq!(vanilla.source, launcher_core::CatalogSource::Installed);
    assert_eq!(vanilla.versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), ["1.12.2"]);
    assert!(launcher.version_catalog(LoaderKind::Quilt).await.unwrap().versions.is_empty());

    // Crear una instancia Fabric sin red usa el loader ya instalado.
    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher.instances_create("Sin red", "1.12.2", LoaderKind::Fabric, None).await.unwrap();
    assert_eq!(instance.loader_version.as_deref(), Some("0.15.0"));
}

/// Cada loader solo ofrece las versiones que realmente soporta (red real). Manual.
#[tokio::test]
#[ignore = "requiere internet"]
async fn real_version_catalog_is_filtered_per_loader() {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default())));
    let ids = |c: launcher_core::VersionCatalog| c.versions.into_iter().map(|v| v.id).collect::<Vec<_>>();

    let vanilla = launcher.version_catalog(LoaderKind::Vanilla).await.unwrap();
    assert_eq!(vanilla.source, launcher_core::CatalogSource::Online);
    let kinds: std::collections::HashSet<_> = vanilla.versions.iter().map(|v| format!("{:?}", v.version_type)).collect();
    println!("tipos en vanilla: {kinds:?}, total {}", vanilla.versions.len());
    assert!(kinds.contains("OldAlpha") && kinds.contains("OldBeta") && kinds.contains("Snapshot") && kinds.contains("Release"));

    let vanilla_ids = ids(vanilla);
    for kind in [LoaderKind::Fabric, LoaderKind::Quilt, LoaderKind::Forge, LoaderKind::NeoForge] {
        let list = ids(launcher.version_catalog(kind).await.unwrap());
        println!("{kind:?}: {} versiones, primeras {:?}", list.len(), &list[..list.len().min(4)]);
        assert!(!list.is_empty() && list.len() < vanilla_ids.len());
    }
    assert!(!ids(launcher.version_catalog(LoaderKind::NeoForge).await.unwrap()).contains(&"1.12.2".to_string()));
    assert!(ids(launcher.version_catalog(LoaderKind::Forge).await.unwrap()).contains(&"1.12.2".to_string()));
}

/// Varias operaciones de mods a la vez sobre la misma instancia no se pisan entre si.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn simultaneous_mod_changes_on_one_instance_are_all_kept() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(dir.path().join("data")), Arc::new(Collector::default())));
    launcher.accounts_create_local("Steve").await.unwrap();
    let instance = launcher
        .instances_create("Mods", "1.20.1", LoaderKind::Fabric, Some("0.15.0".into()))
        .await
        .unwrap();

    let jars: Vec<std::path::PathBuf> = (0..10)
        .map(|i| {
            let path = dir.path().join(format!("mod{i}.jar"));
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default()).unwrap();
            write!(zip, r#"{{"schemaVersion":1,"id":"testmod{i}","version":"1.0.{i}","name":"Test {i}"}}"#).unwrap();
            zip.finish().unwrap();
            path
        })
        .collect();

    let tasks: Vec<_> = jars
        .into_iter()
        .map(|jar| {
            let launcher = launcher.clone();
            let id = instance.id.clone();
            tokio::spawn(async move { launcher.mods_add_local(&id, &jar).await.unwrap() })
        })
        .collect();
    for t in tasks {
        t.await.unwrap();
    }
    assert_eq!(launcher.mods_list(&instance.id).await.unwrap().mods.len(), 10);
}

/// Importa un `.mrpack` real (ruta en `DISORDER_E2E_MRPACK`), lo lanza y comprueba que el juego arranca con sus mods.
/// `DISORDER_E2E_DIR=target/e2e-data DISORDER_E2E_MRPACK=ruta.mrpack cargo test -p launcher-core --test launcher -- --ignored --nocapture real_modpack_import_and_launch`
#[tokio::test]
#[ignore = "requiere internet, un .mrpack en DISORDER_E2E_MRPACK y abre Minecraft unos segundos"]
async fn real_modpack_import_and_launch() {
    let pack = std::path::PathBuf::from(std::env::var("DISORDER_E2E_MRPACK").expect("DISORDER_E2E_MRPACK"));
    let temp;
    let root = match std::env::var("DISORDER_E2E_DIR") {
        Ok(dir) => {
            // El juego corre desde su carpeta: la raiz debe ser absoluta o el classpath no se resuelve.
            std::fs::create_dir_all(&dir).unwrap();
            std::path::absolute(dir).unwrap()
        }
        Err(_) => {
            temp = tempfile::tempdir().unwrap();
            temp.path().to_path_buf()
        }
    };
    let events = Arc::new(Collector::default());
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(&root), events.clone()));
    if launcher.accounts_list().await.unwrap().is_empty() {
        launcher.accounts_create_local("Steve").await.unwrap();
    }

    let instance = launcher.modpack_import(&pack).await.unwrap();
    let mods = launcher.mods_list(&instance.id).await.unwrap();
    println!("instancia {:?} ({} {:?}) con {} mods", instance.name, instance.minecraft_version, instance.loader, mods.mods.len());

    launcher.launch(&instance.id).await.unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2400);
    let first = loop {
        {
            let states = events.states.lock().unwrap();
            if let Some(e) = states.iter().find(|e| matches!(e.state, LaunchState::Running { .. } | LaunchState::Failed { .. } | LaunchState::Exited { .. })) {
                break e.state.clone();
            }
        }
        assert!(std::time::Instant::now() < deadline, "timeout");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    assert!(matches!(first, LaunchState::Running { .. }), "{first:?}");

    tokio::time::sleep(Duration::from_secs(45)).await;
    let output = events.output.lock().unwrap().join("\n");
    println!("salida del juego:\n{}", output.lines().take(80).collect::<Vec<_>>().join("\n"));
    let alive = events.states.lock().unwrap().iter().all(|e| !matches!(e.state, LaunchState::Exited { .. }));
    let _ = launcher.stop(&instance.id);
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;
    assert!(alive, "el juego se cerro solo antes de tiempo");
}

#[tokio::test]
async fn adoption_shares_identical_mods_between_instances_and_duplicates() {
    let (dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let a = launcher.instances_create("A", "1.21", LoaderKind::Fabric, Some("0.16.0".into())).await.unwrap();
    let b = launcher.instances_create("B", "1.21", LoaderKind::Fabric, Some("0.16.0".into())).await.unwrap();
    for inst in [&a, &b] {
        std::fs::write(inst.game_dir.join("mods").join("sodium.jar"), vec![9u8; 2048]).unwrap();
    }

    let first = launcher.store_adopt_instance(&a.id).await.unwrap();
    let second = launcher.store_adopt_instance(&b.id).await.unwrap();
    assert_eq!((first.adopted, first.saved_bytes), (1, 0));
    assert_eq!((second.adopted, second.saved_bytes), (1, 2048));
    // Segunda pasada: nada que hacer.
    assert_eq!(launcher.store_adopt_instance(&a.id).await.unwrap().adopted, 0);

    let objects = artifact_store::ArtifactStore::open(launcher.paths().store_dir()).unwrap().list().unwrap();
    assert_eq!(objects.len(), 1, "un solo objeto fisico para ambas instancias");

    // El almacen vive bajo la raiz de datos, nunca dentro de una instancia.
    assert!(launcher.paths().store_dir().starts_with(dir.path()));
    let copy = launcher.instances_duplicate(&a.id).await.unwrap();
    assert_eq!(std::fs::read(copy.game_dir.join("mods").join("sodium.jar")).unwrap(), vec![9u8; 2048]);
    assert_eq!(launcher.store_adopt_instance(&copy.id).await.unwrap().adopted, 0, "el indice copiado sigue valido");
}

#[tokio::test]
async fn dpack_roundtrip_reuses_the_store_and_leaves_personal_state_out() {
    let (dir, launcher, events) = launcher();
    fake_installation(&launcher, dir.path());
    launcher.accounts_create_local("Steve").await.unwrap();
    let source = launcher.instances_create("Mi pack", "1.99", LoaderKind::Vanilla, None).await.unwrap();
    let game = &source.game_dir;
    std::fs::write(game.join("mods").join("sodium.jar"), vec![5u8; 4096]).unwrap();
    std::fs::write(game.join("mods").join("privado.jar"), vec![6u8; 512]).unwrap();
    std::fs::create_dir_all(game.join("config")).unwrap();
    std::fs::write(game.join("config").join("sodium.toml"), b"quality = 2\n").unwrap();
    std::fs::create_dir_all(game.join("saves").join("Mundo")).unwrap();
    std::fs::write(game.join("saves").join("Mundo").join("level.dat"), b"mundo").unwrap();
    std::fs::write(game.join("options.txt"), b"fov:90").unwrap();

    let out = dir.path().join("pack.7z");
    let summary = launcher.dpack_export(&source.id, &out, false).await.unwrap();
    assert_eq!(summary.files, 3, "dos mods y una config; sin mundos ni options");
    assert!(out.is_file());
    assert!(events.progress.lock().unwrap().iter().any(|e| e.phase == "export7z"));

    let store = artifact_store::ArtifactStore::open(launcher.paths().store_dir()).unwrap();
    let before = store.list().unwrap().len();
    assert_eq!(before, 2, "la exportacion adopta los mods de la instancia");

    let imported = launcher.dpack_import(&out).await.unwrap();
    assert_ne!(imported.id, source.id);
    assert_eq!(imported.name, "Mi pack");
    assert_eq!(std::fs::read(imported.game_dir.join("mods").join("sodium.jar")).unwrap(), vec![5u8; 4096]);
    assert_eq!(std::fs::read(imported.game_dir.join("mods").join("privado.jar")).unwrap(), vec![6u8; 512]);
    assert_eq!(std::fs::read(imported.game_dir.join("config").join("sodium.toml")).unwrap(), b"quality = 2\n");
    assert!(!imported.game_dir.join("saves").join("Mundo").exists());
    assert!(!imported.game_dir.join("options.txt").exists());
    assert_eq!(store.list().unwrap().len(), before, "los mods ya conocidos no crean objetos nuevos");
    assert!(events.progress.lock().unwrap().iter().any(|e| e.phase == "import7z"));
    // El indice queda listo: adoptar de nuevo no tiene nada que hacer.
    assert_eq!(launcher.store_adopt_instance(&imported.id).await.unwrap().adopted, 0);
    assert!(!imported.game_dir.join(".disorder").join("import.pending").exists());
}

#[tokio::test]
async fn dpack_import_of_a_broken_file_creates_no_instance() {
    let (dir, launcher, _) = launcher();
    fake_installation(&launcher, dir.path());
    launcher.accounts_create_local("Steve").await.unwrap();
    let junk = dir.path().join("roto.7z");
    std::fs::write(&junk, b"esto no es un paquete").unwrap();
    assert!(matches!(launcher.dpack_import(&junk).await, Err(LauncherError::InvalidInput(_))));
    assert!(launcher.instances_list().await.unwrap().is_empty());
}

#[tokio::test]
async fn storage_cleanup_frees_only_what_no_instance_uses() {
    let (_dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let a = launcher.instances_create("A", "1.21", LoaderKind::Fabric, Some("0.16.0".into())).await.unwrap();
    let b = launcher.instances_create("B", "1.21", LoaderKind::Fabric, Some("0.16.0".into())).await.unwrap();
    for inst in [&a, &b] {
        std::fs::write(inst.game_dir.join("mods").join("sodium.jar"), vec![9u8; 2048]).unwrap();
    }
    std::fs::write(a.game_dir.join("mods").join("solo-a.jar"), vec![4u8; 100]).unwrap();
    launcher.store_adopt_instance(&a.id).await.unwrap();
    launcher.store_adopt_instance(&b.id).await.unwrap();

    let summary = launcher.store_summary().await.unwrap();
    assert_eq!((summary.store_bytes, summary.shared_bytes, summary.reclaimable_bytes), (2148, 2048, 0));
    // Un objeto que ninguna instancia usa se libera al momento (no hay gracia por fecha; ver ADR-003).
    std::fs::remove_file(a.game_dir.join("mods").join("solo-a.jar")).unwrap();
    launcher.store_adopt_instance(&a.id).await.unwrap();
    assert_eq!(launcher.store_cleanup().await.unwrap().freed_bytes, 100);

    // Borrar una instancia no toca los mods que otra sigue usando.
    launcher.instances_delete(&a.id).await.unwrap();
    assert_eq!(launcher.store_cleanup().await.unwrap().freed_bytes, 0);
    assert_eq!(std::fs::read(b.game_dir.join("mods").join("sodium.jar")).unwrap(), vec![9u8; 2048]);
}

/// Flujo real del almacen comun y de los paquetes `.7z` sobre los datos de la app (con su Minecraft ya instalado):
/// duplica una instancia con mods reales, adopta, exporta, importa y lanza la importada.
/// `DISORDER_E2E_DATA=<datos de la app> DISORDER_E2E_INSTANCE=<id> DISORDER_E2E_OUT=<archivo.7z> cargo test -p launcher-core --test launcher -- --ignored --nocapture real_store_and_dpack_flow`
#[tokio::test]
#[ignore = "usa los datos reales de la app (DISORDER_E2E_DATA), requiere internet y abre Minecraft unos segundos"]
async fn real_store_and_dpack_flow() {
    let root = std::path::PathBuf::from(std::env::var("DISORDER_E2E_DATA").expect("DISORDER_E2E_DATA"));
    let source_id = std::env::var("DISORDER_E2E_INSTANCE").expect("DISORDER_E2E_INSTANCE");
    let out = std::path::PathBuf::from(std::env::var("DISORDER_E2E_OUT").expect("DISORDER_E2E_OUT"));
    let events = Arc::new(Collector::default());
    let launcher = Arc::new(Launcher::new(LauncherPaths::new(&root), events.clone()));

    let source = launcher.instances_list().await.unwrap().into_iter().find(|i| i.id == source_id).expect("instancia");
    let mods_of = |dir: &std::path::Path| -> std::collections::BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(dir.join("mods"))
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap()))
            .collect()
    };
    let original = mods_of(&source.game_dir);
    let logical: usize = original.values().map(|v| v.len()).sum();
    println!("origen: {} mods, {} bytes", original.len(), logical);

    // 1. Duplicar: los jars se enlazan, no se copian.
    let t = std::time::Instant::now();
    let copy = launcher.instances_duplicate(&source_id).await.unwrap();
    println!("duplicar: {:?}", t.elapsed());
    assert_eq!(mods_of(&copy.game_dir), original);

    // 2. Adoptar ambas: un solo objeto fisico por mod.
    let t = std::time::Instant::now();
    let a = launcher.store_adopt_instance(&source_id).await.unwrap();
    println!("adoptar origen: {:?} -> {a:?}", t.elapsed());
    let t = std::time::Instant::now();
    let again = launcher.store_adopt_instance(&source_id).await.unwrap();
    println!("segunda pasada: {:?} -> {again:?}", t.elapsed());
    assert_eq!(again.adopted, 0);
    let b = launcher.store_adopt_instance(&copy.id).await.unwrap();
    println!("adoptar copia: {b:?}");
    let store = artifact_store::ArtifactStore::open(launcher.paths().store_dir()).unwrap();
    let objects = store.list().unwrap();
    println!("objetos en el almacen: {}", objects.len());
    println!("resumen: {:?}", launcher.store_summary().await.unwrap());

    // 3. Exportar y abrirlo como un 7z normal.
    let t = std::time::Instant::now();
    let summary = launcher.dpack_export(&source_id, &out, false).await.unwrap();
    println!("exportar: {:?} -> {summary:?}, archivo {} bytes", t.elapsed(), std::fs::metadata(&out).unwrap().len());

    // 4. Importar: nada nuevo en el almacen y contenido identico.
    let before = store.list().unwrap().len();
    let t = std::time::Instant::now();
    let imported = launcher.dpack_import(&out).await.unwrap();
    println!("importar: {:?} -> {} ({})", t.elapsed(), imported.name, imported.id);
    assert_eq!(store.list().unwrap().len(), before, "los mods ya conocidos no se escriben otra vez");
    assert_eq!(mods_of(&imported.game_dir), original);
    println!("IMPORTED_ID={}", imported.id);
    println!("COPY_ID={}", copy.id);

    // 5. Lanzar la importada.
    launcher.launch(&imported.id).await.unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(1500);
    let first = loop {
        {
            let states = events.states.lock().unwrap();
            if let Some(e) = states.iter().find(|e| matches!(e.state, LaunchState::Running { .. } | LaunchState::Failed { .. } | LaunchState::Exited { .. })) {
                break e.state.clone();
            }
        }
        assert!(std::time::Instant::now() < deadline, "timeout");
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    println!("estado: {first:?}");
    assert!(matches!(first, LaunchState::Running { .. }), "{first:?}");
    tokio::time::sleep(Duration::from_secs(40)).await;
    let output = events.output.lock().unwrap().join("\n");
    let tail: Vec<&str> = output.lines().rev().take(25).collect();
    println!("salida del juego (final):\n{}", tail.into_iter().rev().collect::<Vec<_>>().join("\n"));
    let alive = events.states.lock().unwrap().iter().all(|e| !matches!(e.state, LaunchState::Exited { .. }));
    let _ = launcher.stop(&imported.id);
    wait_for_state(&events, |s| matches!(s, LaunchState::Exited { .. })).await;
    assert!(alive, "el juego se cerro solo antes de tiempo");
    assert!(output.contains("Loading") || output.to_lowercase().contains("fabric"), "el juego debio cargar Fabric y sus mods");
}

#[tokio::test]
async fn adoption_keeps_the_modrinth_origin_known_from_the_mod_lock() {
    let (_dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let inst = launcher.instances_create("A", "1.21", LoaderKind::Fabric, Some("0.16.0".into())).await.unwrap();
    std::fs::write(inst.game_dir.join("mods").join("sodium.jar"), b"sodium").unwrap();
    std::fs::write(inst.game_dir.join("mods").join("privado.jar"), b"privado").unwrap();
    let manager = mods::ModManager::new(&inst.game_dir);
    let mut lock = mods::ModLock::default();
    lock.upsert(mods::LockedMod {
        file: "sodium.jar".into(),
        provider: "modrinth".into(),
        project_id: "AANobbMI".into(),
        version_id: "ver1".into(),
        version_number: "1.0".into(),
        title: "Sodium".into(),
        sha1: "aa".into(),
        installed_at: chrono::Utc::now(),
        dependency_of: None,
        previous: None,
        incompatible: Vec::new(),
    });
    mods::save_lock(&manager.lock_file(), &lock).await.unwrap();

    launcher.store_adopt_instance(&inst.id).await.unwrap();
    let index = artifact_store::ArtifactIndex::load(&inst.game_dir);
    assert_eq!(
        index.entries["mods/sodium.jar"].source,
        artifact_store::ArtifactSource::Modrinth { project_id: "AANobbMI".into(), version_id: "ver1".into() }
    );
    assert_eq!(index.entries["mods/privado.jar"].source, artifact_store::ArtifactSource::Local);
}

/// Un archivo que otro programa mantiene abierto (Java, el antivirus, la ventana de errores de un mod) no debe
/// impedir eliminar una instancia: desaparece de la lista y la carpeta se termina de borrar al liberarse.
#[cfg(windows)]
#[tokio::test]
async fn deleting_an_instance_survives_a_file_held_by_another_program() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_dir, launcher, _) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let inst = launcher.instances_create("Con juego abierto", "1.21", LoaderKind::Vanilla, None).await.unwrap();
    let log = inst.game_dir.join("logs").join("latest.log");
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    std::fs::write(&log, b"x").unwrap();
    std::fs::write(inst.game_dir.join("options.txt"), b"x").unwrap();
    let held = std::fs::OpenOptions::new().write(true).share_mode(3).open(&log).unwrap();

    launcher.instances_delete(&inst.id).await.expect("no debe fallar por un archivo en uso");
    assert!(launcher.instances_list().await.unwrap().is_empty());
    assert!(!inst.game_dir.join("options.txt").exists(), "lo demas se borro");

    drop(held);
    let repo = instance_manager::FileInstanceRepository::new(launcher.paths().instances_dir());
    assert_eq!(repo.remove_leftovers().await, 1);
    assert!(!launcher.paths().instances_dir().join(&inst.id).exists());
}

#[tokio::test]
async fn the_selected_profile_plays_every_instance() {
    let (dir, launcher, _) = launcher();
    let jre = fake_installation(&launcher, dir.path());
    launcher.accounts_create_local("David").await.unwrap();
    let alex = launcher.accounts_create_local("Alex").await.unwrap();
    // Se crea con David (el predeterminado de entonces) y despues se elige a Alex en Perfiles.
    let instance = launcher.instances_create("Base", "1.99", LoaderKind::Vanilla, None).await.unwrap();
    let repo = instance_manager::FileInstanceRepository::new(launcher.paths().instances_dir());
    let instance = repo
        .update(&instance.id, instance_manager::InstanceUpdate { java: Some(JavaSelection::Path { path: jre }), ..Default::default() })
        .await
        .unwrap();
    launcher.accounts_set_default(&alex.id).await.unwrap();

    let plan = launcher.prepare_launch(&instance).await.unwrap();
    let args = plan.game_args.join(" ");
    assert!(args.contains("Alex"), "juega con el perfil elegido, no con el de la creacion: {args}");
    assert!(!args.contains("David"));
}
