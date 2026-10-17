use crate::{AccountRecord, AuthState, IdentityError, IdentityProviderKind, ResolvedIdentity};

#[async_trait::async_trait]
pub trait IdentityProvider: Send + Sync {
    fn kind(&self) -> IdentityProviderKind;
    async fn resolve_identity(
        &self,
        account: &AccountRecord,
    ) -> Result<ResolvedIdentity, IdentityError>;
    async fn refresh(&self, account: &AccountRecord) -> Result<AuthState, IdentityError>;
    async fn logout(&self, account: &AccountRecord) -> Result<(), IdentityError>;
}
