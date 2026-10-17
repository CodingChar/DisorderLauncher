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
}

impl LauncherEvents for Collector {
    fn install_progress(&self, _: InstallProgressEvent) {}
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
    assert!(matches!(launcher.stop("inst_nope"), Err(LauncherError::InstanceNotFound)));
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
async fn instance_crud_validates_profile_and_java() {
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
    assert_eq!((updated.name.as_str(), updated.profile_id.as_str()), ("Renombrada", second.id.as_str()));

    let bad_profile = instance_manager::InstanceUpdate { profile_id: Some("acc_local_nope".into()), ..Default::default() };
    assert!(matches!(launcher.instances_update(&instance.id, bad_profile).await, Err(LauncherError::ProfileMissing)));

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
