use identity_api::*;

pub struct MicrosoftIdentityProvider;

#[async_trait::async_trait]
impl IdentityProvider for MicrosoftIdentityProvider {
    fn kind(&self) -> IdentityProviderKind {
        IdentityProviderKind::Microsoft
    }

    async fn resolve_identity(
        &self,
        _account: &AccountRecord,
    ) -> Result<ResolvedIdentity, IdentityError> {
        Err(IdentityError::MicrosoftNotConfigured)
    }

    async fn refresh(&self, _account: &AccountRecord) -> Result<AuthState, IdentityError> {
        Err(IdentityError::MicrosoftNotConfigured)
    }

    async fn logout(&self, _account: &AccountRecord) -> Result<(), IdentityError> {
        Err(IdentityError::MicrosoftNotConfigured)
    }
}
