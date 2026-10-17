//! Cuentas Microsoft: inicio de sesion en el navegador del sistema, estado y cierre de sesion.
//! Los tokens nunca salen de `identity-microsoft` / el almacen seguro del sistema.

use account_storage::AccountRepository;
use crate::{AuthStateEvent, Launcher, LauncherError};
use identity_api::{
    AccountRecord, AuthState, IdentityError, IdentityProvider, IdentityProviderKind, ProviderAccountData,
    ResolvedIdentity,
};
use identity_microsoft::{MicrosoftIdentityProvider, LOGIN_TIMEOUT};
use instance_manager::InstanceRepository;
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Ranura intercambiable: permite reconstruir el proveedor Microsoft cuando cambia la
/// configuracion sin tocar el registro de proveedores ni a quien ya lo tiene.
pub(crate) struct MicrosoftSlot(RwLock<Arc<MicrosoftIdentityProvider>>);

impl MicrosoftSlot {
    pub(crate) fn new(provider: Arc<MicrosoftIdentityProvider>) -> Self {
        Self(RwLock::new(provider))
    }

    pub(crate) fn replace(&self, provider: Arc<MicrosoftIdentityProvider>) {
        *self.0.write().unwrap_or_else(|p| p.into_inner()) = provider;
    }

    pub(crate) fn current(&self) -> Arc<MicrosoftIdentityProvider> {
        self.0.read().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

#[async_trait::async_trait]
impl IdentityProvider for MicrosoftSlot {
    fn kind(&self) -> IdentityProviderKind {
        IdentityProviderKind::Microsoft
    }

    async fn resolve_identity(&self, account: &AccountRecord) -> Result<ResolvedIdentity, IdentityError> {
        self.current().resolve_identity(account).await
    }

    async fn refresh(&self, account: &AccountRecord) -> Result<AuthState, IdentityError> {
        self.current().refresh(account).await
    }

    async fn logout(&self, account: &AccountRecord) -> Result<(), IdentityError> {
        self.current().logout(account).await
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftStatus {
    /// Hay una app registrada configurada: se puede iniciar sesion.
    pub enabled: bool,
    pub login_in_progress: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginStart {
    /// Direccion que debe abrirse en el navegador del sistema.
    pub auth_url: String,
}

impl Launcher {
    pub fn microsoft_status(&self) -> MicrosoftStatus {
        MicrosoftStatus {
            enabled: self.ms_slot.current().is_enabled(),
            login_in_progress: self.login.lock().unwrap_or_else(|p| p.into_inner()).is_some(),
        }
    }

    /// Prepara el inicio de sesion y devuelve la URL para el navegador. El resultado llega por
    /// `auth://state`: la cuenta se guarda sola al completarse.
    pub async fn microsoft_begin_login(self: &Arc<Self>) -> Result<LoginStart, LauncherError> {
        if self.login.lock().unwrap_or_else(|p| p.into_inner()).is_some() {
            return Err(LauncherError::LoginInProgress);
        }
        let provider = self.ms_slot.current();
        let pending = provider.begin_login().await?;
        let auth_url = pending.auth_url().to_string();

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        *self.login.lock().unwrap_or_else(|p| p.into_inner()) = Some(cancel_tx);
        self.events.auth_state(AuthStateEvent::Authorizing);

        let this = Arc::clone(self);
        tokio::spawn(async move {
            let outcome = tokio::select! {
                _ = cancel_rx => None,
                result = provider.complete_login(pending, LOGIN_TIMEOUT) => Some(result),
            };
            let event = match outcome {
                None => AuthStateEvent::Cancelled,
                Some(Ok(login)) => match this.save_microsoft_account(&provider, login).await {
                    Ok(account) => AuthStateEvent::Connected { account_id: account.id, name: account.label },
                    Err(e) => AuthStateEvent::Failed { message: e.to_string() },
                },
                Some(Err(IdentityError::MicrosoftAuthorizationCancelled)) => AuthStateEvent::Cancelled,
                Some(Err(e)) => AuthStateEvent::Failed { message: LauncherError::from(e).to_string() },
            };
            *this.login.lock().unwrap_or_else(|p| p.into_inner()) = None;
            this.events.auth_state(event);
        });

        Ok(LoginStart { auth_url })
    }

    async fn save_microsoft_account(
        &self,
        provider: &Arc<MicrosoftIdentityProvider>,
        login: identity_microsoft::LoginResult,
    ) -> Result<AccountRecord, LauncherError> {
        // La misma cuenta de Minecraft conserva su id (y las instancias que la usan).
        let uuid = login.profile().id.clone();
        let existing = self.accounts.list().await?.into_iter().find(|a| {
            matches!(&a.provider_data, ProviderAccountData::Microsoft(m) if m.minecraft_uuid.as_deref() == Some(uuid.as_str()))
        });
        let account = provider.finalize_account(existing, login).await?;
        self.accounts.save(&account).await?;
        if self.accounts.get_default().await?.is_none() {
            self.accounts.set_default(&account.id).await?;
        }
        Ok(account)
    }

    /// Cancela el inicio de sesion en curso (cierra el servidor de callback).
    pub fn microsoft_cancel_login(&self) {
        if let Some(cancel) = self.login.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = cancel.send(());
        }
    }

    /// Estado de la cuenta para la interfaz. Para Microsoft puede consultar la red.
    pub async fn accounts_auth_state(&self, id: &str) -> Result<AuthState, LauncherError> {
        let account = self.accounts.get(id).await?.ok_or(LauncherError::AccountNotFound)?;
        let provider = self
            .registry
            .get(account.provider)
            .ok_or_else(|| LauncherError::Identity("proveedor no disponible.".into()))?;
        Ok(provider.refresh(&account).await?)
    }

    /// Quita un perfil. Las instancias que lo usan se lo impiden: nunca quedan sin perfil en silencio.
    pub async fn accounts_remove(&self, id: &str) -> Result<(), LauncherError> {
        let account = self.accounts.get(id).await?.ok_or(LauncherError::AccountNotFound)?;
        let using = self.instances.list().await?.into_iter().filter(|i| i.profile_id == id).count();
        if using > 0 {
            return Err(LauncherError::ProfileInUse(using));
        }

        if let Some(provider) = self.registry.get(account.provider) {
            provider.logout(&account).await?;
        }
        let was_default = self.accounts.get_default().await?.as_deref() == Some(id);
        self.accounts.delete(id).await?;
        if was_default {
            if let Some(next) = self.accounts.list().await?.first() {
                self.accounts.set_default(&next.id).await?;
            }
        }
        Ok(())
    }
}
