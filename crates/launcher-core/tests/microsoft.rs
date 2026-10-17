//! Contratos de Microsoft en el launcher: sesion, errores sin degradar a Offline, perfiles.

use credential_store::{CredentialStore, InMemoryCredentialStore};
use identity_api::{AccountRecord, IdentityError, IdentityProviderKind, MicrosoftAccountMetadata, ProviderAccountData};
use identity_microsoft::{FakeMicrosoftProvider, MicrosoftEndpoints};
use instance_manager::{InstanceRepository, InstanceUpdate, JavaSelection, LoaderKind};
use launcher_core::{
    AuthStateEvent, GameOutputEvent, InstallProgressEvent, Launcher, LauncherError, LauncherEvents, LauncherPaths,
    LaunchStateEvent, OutputStream, Settings,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use test_support::FakeServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const CLIENT_ID: &str = "00000000-0000-0000-0000-000000000002";
const VERSION_JSON: &str = r#"{
  "id": "1.99", "type": "release", "mainClass": "net.minecraft.client.main.Main",
  "downloads": { "client": { "sha1": "aa", "size": 1, "url": "https://example.com/client.jar" } },
  "assetIndex": { "id": "idx", "sha1": "bb", "size": 1, "totalSize": 1, "url": "https://example.com/idx.json" },
  "assets": "idx", "libraries": [],
  "arguments": { "game": ["--username", "${auth_player_name}", "--uuid", "${auth_uuid}", "--userType", "${user_type}"], "jvm": ["-cp", "${classpath}"] }
}"#;

#[derive(Default)]
struct Collector {
    auth: Mutex<Vec<AuthStateEvent>>,
}

impl LauncherEvents for Collector {
    fn install_progress(&self, _: InstallProgressEvent) {}
    fn launch_state(&self, _: LaunchStateEvent) {}
    fn game_output(&self, _: OutputStream, _: GameOutputEvent) {}
    fn auth_state(&self, event: AuthStateEvent) {
        self.auth.lock().unwrap().push(event);
    }
}

fn endpoints_of(server: &FakeServer) -> MicrosoftEndpoints {
    MicrosoftEndpoints {
        authorize: format!("{}/authorize", server.base),
        token: format!("{}/token", server.base),
        xbox_user: format!("{}/xbox", server.base),
        xsts: format!("{}/xsts", server.base),
        minecraft_login: format!("{}/mc/login", server.base),
        minecraft_entitlements: format!("{}/mc/entitlements", server.base),
        minecraft_profile: format!("{}/mc/profile", server.base),
    }
}

struct Env {
    dir: tempfile::TempDir,
    launcher: Arc<Launcher>,
    events: Arc<Collector>,
    store: Arc<InMemoryCredentialStore>,
}

fn env_with(server: Option<&FakeServer>) -> Env {
    let dir = tempfile::tempdir().unwrap();
    let events = Arc::new(Collector::default());
    let store = Arc::new(InMemoryCredentialStore::new());
    let mut launcher = Launcher::new(LauncherPaths::new(dir.path()), events.clone()).with_credential_store(store.clone());
    if let Some(server) = server {
        launcher = launcher.with_microsoft_endpoints(endpoints_of(server));
    }
    Env { dir, launcher: Arc::new(launcher), events, store }
}

async fn enable_microsoft(env: &Env) {
    let settings = Settings { microsoft_client_id: Some(CLIENT_ID.into()), ..Settings::default() };
    env.launcher.settings_update(settings).await.unwrap();
}

async fn browser_callback(redirect_uri: &str, query: &str) {
    let url = url::Url::parse(redirect_uri).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", url.port().unwrap())).await.unwrap();
    stream.write_all(format!("GET {}?{query} HTTP/1.1\r\nHost: localhost\r\n\r\n", url.path()).as_bytes()).await.unwrap();
    let mut sink = String::new();
    let _ = stream.read_to_string(&mut sink).await;
}

async fn wait_auth(env: &Env, pred: impl Fn(&AuthStateEvent) -> bool) -> AuthStateEvent {
    for _ in 0..100 {
        if let Some(e) = env.events.auth.lock().unwrap().iter().find(|e| pred(e)) {
            return e.clone();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("evento esperado no llego: {:?}", env.events.auth.lock().unwrap());
}

/// Login completo contra el servidor falso. Devuelve el id de la cuenta creada.
async fn connect_account(env: &Env) -> String {
    let start = env.launcher.microsoft_begin_login().await.unwrap();
    let q: std::collections::HashMap<String, String> =
        url::Url::parse(&start.auth_url).unwrap().query_pairs().into_owned().collect();
    browser_callback(&q["redirect_uri"], &format!("code=abc&state={}", q["state"])).await;
    match wait_auth(env, |e| matches!(e, AuthStateEvent::Connected { .. } | AuthStateEvent::Failed { .. })).await {
        AuthStateEvent::Connected { account_id, .. } => account_id,
        other => panic!("{other:?}"),
    }
}

/// Deja `1.99` instalada en disco y devuelve la carpeta de un Java falso.
fn fake_installation(launcher: &Launcher, root: &std::path::Path) -> std::path::PathBuf {
    let paths = launcher.paths();
    let descriptor = paths.version_descriptor("1.99");
    std::fs::create_dir_all(descriptor.parent().unwrap()).unwrap();
    std::fs::write(&descriptor, VERSION_JSON).unwrap();
    std::fs::write(descriptor.with_file_name("1.99.jar"), b"jar").unwrap();
    let indexes = paths.minecraft_dir().join("assets").join("indexes");
    std::fs::create_dir_all(&indexes).unwrap();
    std::fs::write(indexes.join("idx.json"), r#"{"objects":{}}"#).unwrap();
    let jre = root.join("fake-jre");
    std::fs::create_dir_all(jre.join("bin")).unwrap();
    std::fs::write(jre.join("bin").join(if cfg!(windows) { "java.exe" } else { "java" }), b"").unwrap();
    jre
}

async fn instance_for(env: &Env, account_id: &str) -> instance_manager::Instance {
    env.launcher.accounts_set_default(account_id).await.unwrap();
    let jre = fake_installation(&env.launcher, env.dir.path());
    let instance = env.launcher.instances_create("Con Microsoft", "1.99", LoaderKind::Vanilla, None).await.unwrap();
    instance_manager::FileInstanceRepository::new(env.launcher.paths().instances_dir())
        .update(&instance.id, InstanceUpdate { java: Some(JavaSelection::Path { path: jre }), ..Default::default() })
        .await
        .unwrap()
}

fn arg_after<'a>(args: &'a [String], flag: &str) -> &'a str {
    &args[args.iter().position(|a| a == flag).unwrap() + 1]
}

#[tokio::test]
async fn microsoft_is_disabled_until_an_app_id_is_configured() {
    let env = env_with(None);
    assert!(!env.launcher.microsoft_status().enabled);
    match env.launcher.microsoft_begin_login().await {
        Err(LauncherError::Identity(msg)) => assert!(msg.contains("no esta configurado"), "{msg}"),
        other => panic!("{other:?}"),
    }

    // Un id invalido se rechaza al guardar y no habilita nada.
    let bad = Settings { microsoft_client_id: Some("no-es-un-guid".into()), ..Settings::default() };
    assert!(matches!(env.launcher.settings_update(bad).await, Err(LauncherError::InvalidInput(_))));
    assert!(!env.launcher.microsoft_status().enabled);

    enable_microsoft(&env).await;
    assert!(env.launcher.microsoft_status().enabled);
}

#[tokio::test]
async fn login_saves_a_secret_free_account_and_launch_uses_its_identity() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let env = env_with(Some(&server));
    enable_microsoft(&env).await;
    assert!(!env.launcher.microsoft_status().login_in_progress);

    let account_id = connect_account(&env).await;
    assert!(!env.launcher.microsoft_status().login_in_progress, "el login pendiente se limpia al terminar");

    let accounts = env.launcher.accounts_list().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].provider, IdentityProviderKind::Microsoft);
    assert_eq!(accounts[0].label, "Notch");
    assert_eq!(env.launcher.accounts_default().await.unwrap().as_deref(), Some(account_id.as_str()));

    let json = serde_json::to_string(&accounts).unwrap();
    assert!(!json.contains("refresh-1") && !json.contains("mc-token"), "sin tokens en el registro de cuentas");
    let on_disk = std::fs::read_to_string(env.dir.path().join("accounts").join(format!("{account_id}.json"))).unwrap();
    assert!(!on_disk.contains("refresh-1") && !on_disk.contains("mc-token"));

    // Una instancia vinculada a la cuenta lanza con la identidad de Microsoft.
    let instance = instance_for(&env, &account_id).await;
    let plan = env.launcher.prepare_launch(&instance).await.unwrap();
    let args = plan.command_args().unwrap();
    assert_eq!(arg_after(&args, "--username"), "Notch");
    assert_eq!(arg_after(&args, "--uuid"), "069a79f444e94726a5befca90e38aaf5");
    assert_eq!(arg_after(&args, "--userType"), "msa");
    assert_eq!(plan.secrets(), ["mc-token-abcdef"], "el token se redacta de la salida del juego");
    assert!(!format!("{plan:?}").contains("mc-token"));
}

#[tokio::test]
async fn expired_session_asks_to_sign_in_and_never_falls_back_to_a_local_profile() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let env = env_with(Some(&server));
    enable_microsoft(&env).await;
    let account_id = connect_account(&env).await;
    let instance = instance_for(&env, &account_id).await;

    // Un perfil local existe, pero la instancia no debe usarlo en silencio.
    env.launcher.accounts_create_local("Steve").await.unwrap();

    // Reiniciar la app (sin sesion en memoria) y que el refresh token ya no sirva.
    let restarted = Launcher::new(LauncherPaths::new(env.dir.path()), env.events.clone())
        .with_credential_store(env.store.clone())
        .with_microsoft_endpoints(endpoints_of(&server));
    restarted.load_settings().await;
    server.respond("POST /token", 400, r#"{"error":"invalid_grant"}"#);

    match restarted.prepare_launch(&instance).await {
        Err(e @ LauncherError::SignInRequired) => assert_eq!(e.kind(), launcher_core::ErrorKind::SignIn),
        other => panic!("{other:?}"),
    }
    // La instancia sigue ligada a la cuenta Microsoft.
    let still = restarted.instances_list().await.unwrap().remove(0);
    assert_eq!(still.profile_id, account_id);

    let state = restarted.accounts_auth_state(&account_id).await.unwrap();
    assert!(matches!(state, identity_api::AuthState::MicrosoftExpired));
    // El token muerto ya no esta en el almacen seguro.
    assert_eq!(env.store.get_secret(&format!("ms-refresh-{account_id}")).await.unwrap(), None);
}

#[tokio::test]
async fn failed_logins_and_cancellation_report_state_and_release_the_slot() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let env = env_with(Some(&server));
    enable_microsoft(&env).await;

    // State incorrecto: falla con un mensaje claro y no guarda nada.
    let start = env.launcher.microsoft_begin_login().await.unwrap();
    assert!(matches!(env.launcher.microsoft_begin_login().await, Err(LauncherError::LoginInProgress)));
    let redirect = url::Url::parse(&start.auth_url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "redirect_uri")
        .unwrap()
        .1
        .into_owned();
    browser_callback(&redirect, "code=stolen&state=attacker").await;
    wait_auth(&env, |e| matches!(e, AuthStateEvent::Failed { .. })).await;
    assert!(env.launcher.accounts_list().await.unwrap().is_empty());
    assert!(server.requests_to("POST /token").is_empty());

    // Cancelar libera el cupo.
    env.events.auth.lock().unwrap().clear();
    env.launcher.microsoft_begin_login().await.unwrap();
    env.launcher.microsoft_cancel_login();
    wait_auth(&env, |e| *e == AuthStateEvent::Cancelled).await;
    assert!(!env.launcher.microsoft_status().login_in_progress);
}

#[tokio::test]
async fn profiles_in_use_cannot_be_removed_and_removal_forgets_secrets() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let env = env_with(Some(&server));
    enable_microsoft(&env).await;
    let account_id = connect_account(&env).await;
    let steve = env.launcher.accounts_create_local("Steve").await.unwrap();
    let instance = instance_for(&env, &account_id).await;

    assert!(matches!(env.launcher.accounts_remove(&account_id).await, Err(LauncherError::ProfileInUse(1))));
    assert!(env.store.get_secret(&format!("ms-refresh-{account_id}")).await.unwrap().is_some());

    env.launcher.instances_delete(&instance.id).await.unwrap();
    env.launcher.accounts_remove(&account_id).await.unwrap();
    assert_eq!(env.store.get_secret(&format!("ms-refresh-{account_id}")).await.unwrap(), None);
    let remaining = env.launcher.accounts_list().await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(env.launcher.accounts_default().await.unwrap().as_deref(), Some(steve.id.as_str()), "el predeterminado pasa a otro perfil");
    assert!(matches!(env.launcher.accounts_remove("acc_nope").await, Err(LauncherError::AccountNotFound)));
}

fn microsoft_account(id: &str) -> AccountRecord {
    AccountRecord {
        id: id.into(),
        provider: IdentityProviderKind::Microsoft,
        label: "Alex".into(),
        created_at: chrono::Utc::now(),
        last_used_at: None,
        provider_data: ProviderAccountData::Microsoft(MicrosoftAccountMetadata {
            minecraft_uuid: Some("uuid-alex".into()),
            minecraft_name: Some("Alex".into()),
            secure_refresh_token_key: Some(format!("ms-refresh-{id}")),
            last_successful_auth: None,
        }),
    }
}

/// Contrato: el motor de lanzamiento acepta cualquier `IdentityProvider`.
#[tokio::test]
async fn launch_works_with_any_identity_provider_and_provider_errors_stay_errors() {
    use account_storage::{AccountRepository, FileAccountRepository};

    let fake = Arc::new(FakeMicrosoftProvider::succeeding("Alex", "11111111222233334444555555555555"));
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default())).with_identity_provider(fake.clone());

    let accounts = FileAccountRepository::new(launcher.paths().accounts_dir());
    accounts.save(&microsoft_account("acc_ms_fake")).await.unwrap();
    accounts.set_default("acc_ms_fake").await.unwrap();

    let jre = fake_installation(&launcher, dir.path());
    let instance = launcher.instances_create("Fake", "1.99", LoaderKind::Vanilla, None).await.unwrap();
    let instance = instance_manager::FileInstanceRepository::new(launcher.paths().instances_dir())
        .update(&instance.id, InstanceUpdate { java: Some(JavaSelection::Path { path: jre.clone() }), ..Default::default() })
        .await
        .unwrap();

    let plan = launcher.prepare_launch(&instance).await.unwrap();
    assert_eq!(arg_after(&plan.command_args().unwrap(), "--username"), "Alex");
    assert_eq!(fake.resolve_calls(), 1);

    // Si el proveedor falla, el lanzamiento falla con el error del proveedor: no hay degradacion a Local.
    let failing = Arc::new(FakeMicrosoftProvider::failing(|| IdentityError::MicrosoftReauthRequired));
    let launcher2 = Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Collector::default())).with_identity_provider(failing.clone());
    launcher2.accounts_create_local("Steve").await.unwrap();
    assert!(matches!(launcher2.prepare_launch(&instance).await, Err(LauncherError::SignInRequired)));
    assert_eq!(failing.resolve_calls(), 1);
    let unchanged = launcher2.instances_list().await.unwrap().remove(0);
    assert_eq!(unchanged.profile_id, "acc_ms_fake");
}
