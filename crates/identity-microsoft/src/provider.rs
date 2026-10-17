use crate::{
    challenge_for, generate_state, generate_verifier, AuthStepError, CallbackError, CallbackServer, ConfigError,
    McProfile, McSession, MicrosoftAuthClient, MicrosoftConfig, Secret,
};
use chrono::Utc;
use credential_store::CredentialStore;
use identity_api::{
    AccountRecord, AuthErrorKind, AuthState, GameSessionMaterial, IdentityError, IdentityProvider,
    IdentityProviderKind, MicrosoftAccountMetadata, ProviderAccountData, ResolvedIdentity, SkinReference,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Valor de `${user_type}` para cuentas Microsoft.
const USER_TYPE: &str = "msa";
/// Se renueva la sesion un poco antes de que expire.
const EXPIRY_MARGIN: Duration = Duration::from_secs(300);
const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(3600);
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

struct CachedSession {
    access_token: Secret,
    expires_at: Instant,
    profile: McProfile,
}

pub struct MicrosoftIdentityProvider {
    auth: Option<Arc<MicrosoftAuthClient>>,
    store: Arc<dyn CredentialStore>,
    /// Sesiones de Minecraft en memoria (nunca en disco).
    cache: Mutex<HashMap<String, CachedSession>>,
    /// Un candado por cuenta: solo una renovacion a la vez (single-flight).
    locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

/// Login en curso: el navegador ya recibio `auth_url`; el servidor local espera el callback.
pub struct PendingLogin {
    auth_url: String,
    redirect_uri: String,
    state: String,
    verifier: Secret,
    server: CallbackServer,
}

impl PendingLogin {
    /// URL que debe abrirse en el navegador del sistema.
    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }
}

impl std::fmt::Debug for PendingLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingLogin").field("redirect_uri", &self.redirect_uri).finish_non_exhaustive()
    }
}

/// Resultado de un login correcto, antes de guardarlo como cuenta.
pub struct LoginResult {
    refresh_token: Secret,
    session: McSession,
}

impl LoginResult {
    pub fn profile(&self) -> &McProfile {
        &self.session.profile
    }
}

impl MicrosoftIdentityProvider {
    pub fn new(http: reqwest::Client, config: MicrosoftConfig, store: Arc<dyn CredentialStore>) -> Result<Self, ConfigError> {
        Ok(Self {
            auth: MicrosoftAuthClient::new(http, config)?.map(Arc::new),
            store,
            cache: Mutex::new(HashMap::new()),
            locks: Mutex::new(HashMap::new()),
        })
    }

    /// Proveedor sin client id: responde `MicrosoftNotConfigured`, pero permite cerrar sesion.
    pub fn disabled(store: Arc<dyn CredentialStore>) -> Self {
        Self { auth: None, store, cache: Mutex::new(HashMap::new()), locks: Mutex::new(HashMap::new()) }
    }

    pub fn is_enabled(&self) -> bool {
        self.auth.is_some()
    }

    fn client(&self) -> Result<&Arc<MicrosoftAuthClient>, IdentityError> {
        self.auth.as_ref().ok_or(IdentityError::MicrosoftNotConfigured)
    }

    fn refresh_key(account_id: &str) -> String {
        format!("ms-refresh-{account_id}")
    }

    // ---- Login (Authorization Code + PKCE) ------------------------------------------------

    pub async fn begin_login(&self) -> Result<PendingLogin, IdentityError> {
        let client = self.client()?;
        let server = CallbackServer::bind().await.map_err(|e| IdentityError::Other(format!("no se pudo abrir el callback local: {e}")))?;
        let verifier = generate_verifier();
        let state = generate_state();
        let redirect_uri = server.redirect_uri();
        let auth_url = client.authorize_url(&redirect_uri, &state, &challenge_for(verifier.expose()));
        Ok(PendingLogin { auth_url, redirect_uri, state, verifier, server })
    }

    /// Espera el callback, canjea el codigo y recorre la cadena hasta el perfil de Minecraft.
    pub async fn complete_login(&self, pending: PendingLogin, timeout: Duration) -> Result<LoginResult, IdentityError> {
        let client = self.client()?;
        let PendingLogin { redirect_uri, state, verifier, server, .. } = pending;

        let code = server.wait_for_code(&state, timeout).await.map_err(|e| match e {
            CallbackError::Denied(_) => IdentityError::MicrosoftAuthorizationCancelled,
            CallbackError::StateMismatch => IdentityError::MicrosoftStateMismatch,
            CallbackError::TimedOut => IdentityError::MicrosoftLoginTimedOut,
            CallbackError::Io(msg) => IdentityError::Other(msg),
        })?;

        let tokens = client.exchange_code(&code, &redirect_uri, &verifier).await.map_err(map_step_error)?;
        let refresh_token = tokens
            .refresh_token
            .ok_or_else(|| IdentityError::MicrosoftTokenExchangeFailed("sin token de renovacion".into()))?;
        let session = client.minecraft_session(&tokens.access_token).await.map_err(map_step_error)?;
        Ok(LoginResult { refresh_token, session })
    }

    /// Guarda el refresh token en el almacen seguro y devuelve el registro de cuenta (sin secretos).
    /// Si `existing` es la misma cuenta de Minecraft, se reutiliza su id.
    pub async fn finalize_account(&self, existing: Option<AccountRecord>, login: LoginResult) -> Result<AccountRecord, IdentityError> {
        let id = existing
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_else(|| format!("acc_ms_{}", uuid::Uuid::new_v4().simple()));
        let key = Self::refresh_key(&id);
        self.store
            .put_secret(&key, login.refresh_token.expose())
            .await
            .map_err(|e| IdentityError::SecureStoreUnavailable(e.to_string()))?;

        let profile = login.session.profile.clone();
        self.cache_session(&id, &login.session);

        Ok(AccountRecord {
            id,
            provider: IdentityProviderKind::Microsoft,
            label: profile.name.clone(),
            created_at: existing.as_ref().map(|a| a.created_at).unwrap_or_else(Utc::now),
            last_used_at: existing.and_then(|a| a.last_used_at),
            provider_data: ProviderAccountData::Microsoft(MicrosoftAccountMetadata {
                minecraft_uuid: Some(profile.id),
                minecraft_name: Some(profile.name),
                secure_refresh_token_key: Some(key),
                last_successful_auth: Some(Utc::now()),
            }),
        })
    }

    // ---- Sesion -----------------------------------------------------------------------------

    fn cache_session(&self, account_id: &str, session: &McSession) {
        let ttl = if session.expires_in == 0 { DEFAULT_SESSION_TTL } else { Duration::from_secs(session.expires_in) };
        self.cache.lock().unwrap_or_else(|p| p.into_inner()).insert(
            account_id.to_string(),
            CachedSession {
                access_token: session.access_token.clone(),
                expires_at: Instant::now() + ttl,
                profile: session.profile.clone(),
            },
        );
    }

    fn cached_identity(&self, account: &AccountRecord) -> Option<ResolvedIdentity> {
        let cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        let cached = cache.get(&account.id)?;
        (cached.expires_at > Instant::now() + EXPIRY_MARGIN).then(|| identity_from(account, &cached.access_token, &cached.profile))
    }

    fn account_lock(&self, account_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        self.locks.lock().unwrap_or_else(|p| p.into_inner()).entry(account_id.to_string()).or_default().clone()
    }
}

fn identity_from(account: &AccountRecord, token: &Secret, profile: &McProfile) -> ResolvedIdentity {
    ResolvedIdentity {
        profile_id: account.id.clone(),
        provider: IdentityProviderKind::Microsoft,
        display_name: profile.name.clone(),
        game_uuid: profile.id.clone(),
        session: GameSessionMaterial { access_token: token.expose().to_string(), token_type: USER_TYPE.to_string() },
        skin: profile.skin_id.clone().map(|id| SkinReference { id, path: None }),
    }
}

pub(crate) fn map_step_error(error: AuthStepError) -> IdentityError {
    match error {
        AuthStepError::Network(e) => {
            tracing::warn!(error = %e.without_url(), "sin conexion con los servicios de Microsoft");
            IdentityError::NetworkUnavailable
        }
        AuthStepError::InvalidGrant => IdentityError::MicrosoftReauthRequired,
        AuthStepError::TokenExchange(detail) => IdentityError::MicrosoftTokenExchangeFailed(detail),
        AuthStepError::Xbox(detail) => IdentityError::MicrosoftXboxAuthFailed(detail),
        AuthStepError::Xsts(message) => IdentityError::MicrosoftXstsFailed(message),
        AuthStepError::ServicesRejected => IdentityError::MinecraftServicesRejectedApp,
        AuthStepError::NoOwnership | AuthStepError::NoProfile => IdentityError::MinecraftOwnershipUnavailable,
        AuthStepError::Malformed(what) => IdentityError::MicrosoftTokenExchangeFailed(format!("respuesta inesperada ({what})")),
    }
}

#[async_trait::async_trait]
impl IdentityProvider for MicrosoftIdentityProvider {
    fn kind(&self) -> IdentityProviderKind {
        IdentityProviderKind::Microsoft
    }

    async fn resolve_identity(&self, account: &AccountRecord) -> Result<ResolvedIdentity, IdentityError> {
        let client = self.client()?;
        let ProviderAccountData::Microsoft(metadata) = &account.provider_data else {
            return Err(IdentityError::Other("la cuenta no es de Microsoft".into()));
        };

        let lock = self.account_lock(&account.id);
        let _guard = lock.lock().await;
        // Otra tarea pudo renovar mientras esperabamos el candado.
        if let Some(identity) = self.cached_identity(account) {
            return Ok(identity);
        }

        let key = metadata.secure_refresh_token_key.as_deref().ok_or(IdentityError::MicrosoftReauthRequired)?;
        let stored = self
            .store
            .get_secret(key)
            .await
            .map_err(|e| IdentityError::SecureStoreUnavailable(e.to_string()))?
            .ok_or(IdentityError::MicrosoftReauthRequired)?;

        let tokens = match client.refresh(&Secret::new(stored)).await {
            Ok(tokens) => tokens,
            Err(AuthStepError::InvalidGrant) => {
                // Revocado o vencido: ya no sirve y hay que volver a iniciar sesion.
                let _ = self.store.delete_secret(key).await;
                return Err(IdentityError::MicrosoftReauthRequired);
            }
            // Un error transitorio (red, servidor) conserva el token.
            Err(other) => return Err(map_step_error(other)),
        };

        // Microsoft rota el refresh token: guardar el nuevo antes de seguir.
        if let Some(new_refresh) = &tokens.refresh_token {
            self.store
                .put_secret(key, new_refresh.expose())
                .await
                .map_err(|e| IdentityError::SecureStoreUnavailable(e.to_string()))?;
        }

        let session = client.minecraft_session(&tokens.access_token).await.map_err(map_step_error)?;
        self.cache_session(&account.id, &session);
        Ok(identity_from(account, &session.access_token, &session.profile))
    }

    async fn refresh(&self, account: &AccountRecord) -> Result<AuthState, IdentityError> {
        match self.resolve_identity(account).await {
            Ok(_) => Ok(AuthState::MicrosoftAuthenticated),
            Err(IdentityError::MicrosoftReauthRequired) => Ok(AuthState::MicrosoftExpired),
            Err(IdentityError::NetworkUnavailable) => {
                let known = matches!(&account.provider_data, ProviderAccountData::Microsoft(m) if m.minecraft_name.is_some());
                Ok(if known {
                    AuthState::MicrosoftOfflineCachedIdentity
                } else {
                    AuthState::Error(AuthErrorKind::NetworkUnavailable)
                })
            }
            Err(IdentityError::MicrosoftNotConfigured) => Ok(AuthState::Error(AuthErrorKind::ProviderNotConfigured)),
            Err(IdentityError::SecureStoreUnavailable(_)) => Ok(AuthState::Error(AuthErrorKind::SecureStoreUnavailable)),
            Err(other) => Ok(AuthState::Error(AuthErrorKind::Unknown(other.to_string()))),
        }
    }

    /// Olvida los tokens de esta cuenta. Funciona aunque el proveedor este deshabilitado.
    async fn logout(&self, account: &AccountRecord) -> Result<(), IdentityError> {
        if let ProviderAccountData::Microsoft(metadata) = &account.provider_data {
            if let Some(key) = &metadata.secure_refresh_token_key {
                self.store
                    .delete_secret(key)
                    .await
                    .map_err(|e| IdentityError::SecureStoreUnavailable(e.to_string()))?;
            }
        }
        self.cache.lock().unwrap_or_else(|p| p.into_inner()).remove(&account.id);
        Ok(())
    }
}
