use credential_store::{CredentialStore, InMemoryCredentialStore};
use identity_api::{AuthErrorKind, AuthState, IdentityError, IdentityProvider, ProviderAccountData};
use identity_microsoft::{challenge_for, MicrosoftConfig, MicrosoftIdentityProvider};
use std::sync::Arc;
use std::time::Duration;
use test_support::FakeServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const CLIENT_ID: &str = "00000000-0000-0000-0000-000000000001";

fn endpoints_of(server: &FakeServer) -> identity_microsoft::MicrosoftEndpoints {
    identity_microsoft::MicrosoftEndpoints {
        authorize: format!("{}/authorize", server.base),
        token: format!("{}/token", server.base),
        xbox_user: format!("{}/xbox", server.base),
        xsts: format!("{}/xsts", server.base),
        minecraft_login: format!("{}/mc/login", server.base),
        minecraft_entitlements: format!("{}/mc/entitlements", server.base),
        minecraft_profile: format!("{}/mc/profile", server.base),
    }
}

fn provider(server: &FakeServer, store: Arc<InMemoryCredentialStore>) -> MicrosoftIdentityProvider {
    let config = MicrosoftConfig { client_id: Some(CLIENT_ID.into()), endpoints: endpoints_of(server) };
    MicrosoftIdentityProvider::new(reqwest::Client::new(), config, store).unwrap()
}

/// Simula al navegador: abre la URL de callback con los parametros dados.
async fn browser_callback(redirect_uri: String, query: String) {
    let url = url::Url::parse(&redirect_uri).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", url.port().unwrap())).await.unwrap();
    stream.write_all(format!("GET {}?{query} HTTP/1.1\r\nHost: localhost\r\n\r\n", url.path()).as_bytes()).await.unwrap();
    let mut sink = String::new();
    let _ = stream.read_to_string(&mut sink).await;
}

fn query_of(auth_url: &str) -> std::collections::HashMap<String, String> {
    url::Url::parse(auth_url).unwrap().query_pairs().into_owned().collect()
}

/// Login completo y devuelve la cuenta guardada.
async fn login(p: &MicrosoftIdentityProvider) -> identity_api::AccountRecord {
    let pending = p.begin_login().await.unwrap();
    let q = query_of(pending.auth_url());
    let (redirect, state) = (q["redirect_uri"].clone(), q["state"].clone());
    let (waiter, _) = tokio::join!(
        p.complete_login(pending, Duration::from_secs(10)),
        browser_callback(redirect, format!("code=auth-code&state={state}"))
    );
    let result = waiter.unwrap();
    p.finalize_account(None, result).await.unwrap()
}

#[tokio::test]
async fn full_login_links_pkce_stores_only_the_refresh_token_and_resolves_identity() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let p = provider(&server, store.clone());

    let pending = p.begin_login().await.unwrap();
    let q = query_of(pending.auth_url());
    assert_eq!(q["code_challenge_method"], "S256");
    let challenge = q["code_challenge"].clone();
    let (redirect, state) = (q["redirect_uri"].clone(), q["state"].clone());
    let (result, _) = tokio::join!(
        p.complete_login(pending, Duration::from_secs(10)),
        browser_callback(redirect.clone(), format!("code=auth-code&state={state}"))
    );
    let account = p.finalize_account(None, result.unwrap()).await.unwrap();

    // PKCE: el verifier enviado al canjear el codigo corresponde al challenge de la URL.
    let exchange = &server.requests_to("POST /token")[0].body;
    let form: std::collections::HashMap<String, String> = url::form_urlencoded::parse(exchange.as_bytes()).into_owned().collect();
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "auth-code");
    assert_eq!(form["client_id"], CLIENT_ID);
    assert!(!form.contains_key("client_secret"));
    assert_eq!(challenge_for(&form["code_verifier"]), challenge);

    // Cadena de Xbox: el formato de cada solicitud.
    assert!(server.requests_to("POST /xbox")[0].body.contains("d=ms-access"));
    assert!(server.requests_to("POST /xsts")[0].body.contains("rp://api.minecraftservices.com/"));
    assert!(server.requests_to("POST /mc/login")[0].body.contains("XBL3.0 x=uhs-1;xbl-token"));
    assert_eq!(server.requests_to("GET /mc/profile")[0].authorization.as_deref(), Some("Bearer mc-token-abcdef"));

    // La cuenta guardada no contiene tokens; el refresh token esta en el almacen seguro.
    let json = serde_json::to_string(&account).unwrap();
    for secret in ["refresh-1", "ms-access", "mc-token", "xbl-token"] {
        assert!(!json.contains(secret), "{secret} no debe estar en el registro de la cuenta");
    }
    let ProviderAccountData::Microsoft(meta) = &account.provider_data else { panic!() };
    assert_eq!(meta.minecraft_name.as_deref(), Some("Notch"));
    let key = meta.secure_refresh_token_key.clone().unwrap();
    assert_eq!(store.get_secret(&key).await.unwrap().as_deref(), Some("refresh-1"));

    // Resolver usa la sesion en memoria: sin nuevas peticiones.
    let identity = p.resolve_identity(&account).await.unwrap();
    assert_eq!(identity.display_name, "Notch");
    assert_eq!(identity.game_uuid, "069a79f444e94726a5befca90e38aaf5");
    assert_eq!(identity.session.token_type, "msa");
    assert_eq!(identity.session.access_token, "mc-token-abcdef");
    assert_eq!(identity.skin.unwrap().id, "skin-1");
    assert_eq!(server.requests_to("POST /token").len(), 1);
}

#[tokio::test]
async fn restart_refreshes_rotates_the_token_and_is_single_flight() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let account = login(&provider(&server, store.clone())).await;
    let ProviderAccountData::Microsoft(meta) = &account.provider_data else { panic!() };
    let key = meta.secure_refresh_token_key.clone().unwrap();

    // "Reinicio": proveedor nuevo sin cache, y Microsoft rota el refresh token.
    server.respond("POST /token", 200, r#"{"access_token":"ms-access-2","refresh_token":"refresh-2","expires_in":3600}"#);
    let fresh = Arc::new(provider(&server, store.clone()));

    let tasks: Vec<_> = (0..6)
        .map(|_| {
            let (p, a) = (fresh.clone(), account.clone());
            tokio::spawn(async move { p.resolve_identity(&a).await })
        })
        .collect();
    for t in tasks {
        assert_eq!(t.await.unwrap().unwrap().display_name, "Notch");
    }

    let refreshes: Vec<_> = server
        .requests_to("POST /token")
        .into_iter()
        .filter(|r| r.body.contains("grant_type=refresh_token"))
        .collect();
    assert_eq!(refreshes.len(), 1, "solo una renovacion para 6 consultas simultaneas");
    assert!(refreshes[0].body.contains("refresh_token=refresh-1"));
    assert_eq!(store.get_secret(&key).await.unwrap().as_deref(), Some("refresh-2"), "se guarda el token rotado");
}

#[tokio::test]
async fn invalid_grant_requires_reauth_and_forgets_the_dead_token() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let account = login(&provider(&server, store.clone())).await;
    let ProviderAccountData::Microsoft(meta) = &account.provider_data else { panic!() };

    server.respond("POST /token", 400, r#"{"error":"invalid_grant","error_description":"expired"}"#);
    let fresh = provider(&server, store.clone());
    assert!(matches!(fresh.resolve_identity(&account).await, Err(IdentityError::MicrosoftReauthRequired)));
    assert_eq!(store.get_secret(meta.secure_refresh_token_key.as_ref().unwrap()).await.unwrap(), None);
    assert!(matches!(fresh.refresh(&account).await.unwrap(), AuthState::MicrosoftExpired));
}

#[tokio::test]
async fn transient_failures_keep_the_refresh_token() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let account = login(&provider(&server, store.clone())).await;
    let ProviderAccountData::Microsoft(meta) = &account.provider_data else { panic!() };
    let key = meta.secure_refresh_token_key.clone().unwrap();

    // Servidor con error 503.
    server.respond("POST /token", 503, "{}");
    let fresh = provider(&server, store.clone());
    assert!(matches!(fresh.resolve_identity(&account).await, Err(IdentityError::MicrosoftTokenExchangeFailed(_))));
    assert_eq!(store.get_secret(&key).await.unwrap().as_deref(), Some("refresh-1"));

    // Sin conexion en absoluto.
    let offline = MicrosoftIdentityProvider::new(
        reqwest::Client::new(),
        MicrosoftConfig {
            client_id: Some(CLIENT_ID.into()),
            endpoints: identity_microsoft::MicrosoftEndpoints {
                token: "http://127.0.0.1:1/token".into(),
                ..endpoints_of(&server)
            },
        },
        store.clone(),
    )
    .unwrap();
    assert!(matches!(offline.resolve_identity(&account).await, Err(IdentityError::NetworkUnavailable)));
    assert_eq!(store.get_secret(&key).await.unwrap().as_deref(), Some("refresh-1"), "la red caida no borra tokens");
    assert!(matches!(offline.refresh(&account).await.unwrap(), AuthState::MicrosoftOfflineCachedIdentity));
}

#[tokio::test]
async fn xbox_and_minecraft_failures_are_distinct_human_errors() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let account = login(&provider(&server, store.clone())).await;
    let fresh = || provider(&server, store.clone());

    server.respond("POST /xsts", 401, r#"{"XErr":2148916233}"#);
    match fresh().resolve_identity(&account).await {
        Err(IdentityError::MicrosoftXstsFailed(msg)) => assert!(msg.contains("perfil de Xbox"), "{msg}"),
        other => panic!("{other:?}"),
    }
    server.respond("POST /xsts", 401, r#"{"XErr":2148916238}"#);
    match fresh().resolve_identity(&account).await {
        Err(IdentityError::MicrosoftXstsFailed(msg)) => assert!(msg.contains("familia"), "{msg}"),
        other => panic!("{other:?}"),
    }

    server.happy_chain("refresh-1");
    server.respond("POST /mc/login", 403, "{}");
    assert!(matches!(fresh().resolve_identity(&account).await, Err(IdentityError::MinecraftServicesRejectedApp)));

    server.happy_chain("refresh-1");
    server.respond("GET /mc/entitlements", 200, r#"{"items":[]}"#);
    assert!(matches!(fresh().resolve_identity(&account).await, Err(IdentityError::MinecraftOwnershipUnavailable)));

    server.happy_chain("refresh-1");
    server.respond("GET /mc/profile", 404, "{}");
    assert!(matches!(fresh().resolve_identity(&account).await, Err(IdentityError::MinecraftOwnershipUnavailable)));
}

#[tokio::test]
async fn state_mismatch_and_denial_abort_before_any_token_request() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let p = provider(&server, Arc::new(InMemoryCredentialStore::new()));

    let pending = p.begin_login().await.unwrap();
    let redirect = query_of(pending.auth_url())["redirect_uri"].clone();
    let (result, _) = tokio::join!(
        p.complete_login(pending, Duration::from_secs(10)),
        browser_callback(redirect.clone(), "code=stolen&state=attacker".into())
    );
    assert!(matches!(result, Err(IdentityError::MicrosoftStateMismatch)));

    let pending = p.begin_login().await.unwrap();
    let q = query_of(pending.auth_url());
    let (result, _) = tokio::join!(
        p.complete_login(pending, Duration::from_secs(10)),
        browser_callback(q["redirect_uri"].clone(), format!("error=access_denied&state={}", q["state"]))
    );
    assert!(matches!(result, Err(IdentityError::MicrosoftAuthorizationCancelled)));

    assert!(server.requests_to("POST /token").is_empty(), "sin callback valido no se pide ningun token");
}

#[tokio::test]
async fn login_times_out() {
    let server = FakeServer::start().await;
    let p = provider(&server, Arc::new(InMemoryCredentialStore::new()));
    let pending = p.begin_login().await.unwrap();
    assert!(matches!(p.complete_login(pending, Duration::from_millis(150)).await, Err(IdentityError::MicrosoftLoginTimedOut)));
}

#[tokio::test]
async fn relogin_of_the_same_minecraft_account_reuses_the_account_id() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let p = provider(&server, store.clone());
    let first = login(&p).await;

    server.happy_chain("refresh-9");
    let pending = p.begin_login().await.unwrap();
    let q = query_of(pending.auth_url());
    let (result, _) = tokio::join!(
        p.complete_login(pending, Duration::from_secs(10)),
        browser_callback(q["redirect_uri"].clone(), format!("code=c2&state={}", q["state"]))
    );
    let again = p.finalize_account(Some(first.clone()), result.unwrap()).await.unwrap();
    assert_eq!(again.id, first.id);
    assert_eq!(again.created_at, first.created_at);
    let ProviderAccountData::Microsoft(meta) = &again.provider_data else { panic!() };
    assert_eq!(store.get_secret(meta.secure_refresh_token_key.as_ref().unwrap()).await.unwrap().as_deref(), Some("refresh-9"));
}

#[tokio::test]
async fn logout_forgets_tokens_even_when_the_provider_is_disabled() {
    let server = FakeServer::start().await;
    server.happy_chain("refresh-1");
    let store = Arc::new(InMemoryCredentialStore::new());
    let account = login(&provider(&server, store.clone())).await;
    let ProviderAccountData::Microsoft(meta) = &account.provider_data else { panic!() };
    let key = meta.secure_refresh_token_key.clone().unwrap();

    let disabled = MicrosoftIdentityProvider::disabled(store.clone());
    assert!(!disabled.is_enabled());
    assert!(matches!(disabled.resolve_identity(&account).await, Err(IdentityError::MicrosoftNotConfigured)));
    assert!(matches!(disabled.begin_login().await, Err(IdentityError::MicrosoftNotConfigured)));
    assert!(matches!(disabled.refresh(&account).await.unwrap(), AuthState::Error(AuthErrorKind::ProviderNotConfigured)));

    disabled.logout(&account).await.unwrap();
    assert_eq!(store.get_secret(&key).await.unwrap(), None);
}

#[tokio::test]
async fn insecure_remote_endpoints_are_refused_at_construction() {
    let config = MicrosoftConfig {
        client_id: Some(CLIENT_ID.into()),
        endpoints: identity_microsoft::MicrosoftEndpoints { token: "http://login.example.com/token".into(), ..Default::default() },
    };
    assert!(MicrosoftIdentityProvider::new(reqwest::Client::new(), config, Arc::new(InMemoryCredentialStore::new())).is_err());
}
