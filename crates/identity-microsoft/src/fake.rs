use identity_api::{
    AccountRecord, AuthState, GameSessionMaterial, IdentityError, IdentityProvider, IdentityProviderKind,
    ResolvedIdentity,
};
use std::sync::atomic::{AtomicUsize, Ordering};

type Outcome = Box<dyn Fn() -> Result<ResolvedIdentity, IdentityError> + Send + Sync>;

/// Proveedor Microsoft simulado para pruebas de contrato: demuestra que el launcher, las
/// instancias y el lanzamiento funcionan con *cualquier* `IdentityProvider` y que los errores
/// del proveedor no se convierten en perfiles locales.
pub struct FakeMicrosoftProvider {
    outcome: Outcome,
    resolve_calls: AtomicUsize,
}

impl FakeMicrosoftProvider {
    pub fn succeeding(name: &str, uuid: &str) -> Self {
        let (name, uuid) = (name.to_string(), uuid.to_string());
        Self::with_outcome(Box::new(move || {
            Ok(ResolvedIdentity {
                profile_id: String::new(),
                provider: IdentityProviderKind::Microsoft,
                display_name: name.clone(),
                game_uuid: uuid.clone(),
                session: GameSessionMaterial { access_token: "fake-ms-token-0123456789".into(), token_type: "msa".into() },
                skin: None,
            })
        }))
    }

    pub fn failing(error: fn() -> IdentityError) -> Self {
        Self::with_outcome(Box::new(move || Err(error())))
    }

    pub fn with_outcome(outcome: Outcome) -> Self {
        Self { outcome, resolve_calls: AtomicUsize::new(0) }
    }

    pub fn resolve_calls(&self) -> usize {
        self.resolve_calls.load(Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl IdentityProvider for FakeMicrosoftProvider {
    fn kind(&self) -> IdentityProviderKind {
        IdentityProviderKind::Microsoft
    }

    async fn resolve_identity(&self, account: &AccountRecord) -> Result<ResolvedIdentity, IdentityError> {
        self.resolve_calls.fetch_add(1, Ordering::SeqCst);
        (self.outcome)().map(|mut identity| {
            identity.profile_id = account.id.clone();
            identity
        })
    }

    async fn refresh(&self, _account: &AccountRecord) -> Result<AuthState, IdentityError> {
        Ok(AuthState::MicrosoftAuthenticated)
    }

    async fn logout(&self, _account: &AccountRecord) -> Result<(), IdentityError> {
        Ok(())
    }
}
