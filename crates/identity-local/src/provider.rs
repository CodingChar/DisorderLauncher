use identity_api::*;

pub struct LocalIdentityProvider;

#[async_trait::async_trait]
impl IdentityProvider for LocalIdentityProvider {
    fn kind(&self) -> IdentityProviderKind {
        IdentityProviderKind::Local
    }

    async fn resolve_identity(
        &self,
        account: &AccountRecord,
    ) -> Result<ResolvedIdentity, IdentityError> {
        let local_data = match &account.provider_data {
            ProviderAccountData::Local(data) => data,
            _ => {
                return Err(IdentityError::LocalProfileInvalid(
                    "account is not a local profile".into(),
                ))
            }
        };

        Ok(ResolvedIdentity {
            profile_id: account.id.clone(),
            provider: IdentityProviderKind::Local,
            display_name: local_data.display_name.clone(),
            game_uuid: local_data.local_uuid.clone(),
            session: GameSessionMaterial {
                access_token: "0".into(),
                token_type: "legacy".into(),
            },
            skin: local_data
                .skin_id
                .as_ref()
                .map(|id| SkinReference { id: id.clone(), path: None }),
        })
    }

    async fn refresh(&self, _account: &AccountRecord) -> Result<AuthState, IdentityError> {
        Ok(AuthState::LocalReady)
    }

    async fn logout(&self, _account: &AccountRecord) -> Result<(), IdentityError> {
        Ok(())
    }
}
