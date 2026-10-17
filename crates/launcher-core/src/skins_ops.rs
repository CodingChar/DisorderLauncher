//! Biblioteca de skins locales y su vinculo con perfiles Offline Gaming.
//! La skin local se guarda y previsualiza; no se afirma que este registrada en Microsoft
//! ni que se muestre dentro del juego sin un mecanismo compatible (spec 03 §12, spec 10).

use crate::{Launcher, LauncherError};
use identity_api::{IdentityProviderKind, ProviderAccountData};
use skins::{SkinAsset, SkinError, SkinModel};
use std::path::Path;

impl From<SkinError> for LauncherError {
    fn from(e: SkinError) -> Self {
        tracing::warn!(error = %e, "skin rechazada");
        match e {
            SkinError::NotFound => Self::InvalidInput("No se encontro esa skin.".into()),
            SkinError::Io(_) | SkinError::Json(_) | SkinError::Encode => Self::Storage,
            other => Self::InvalidInput(sentence(&other.to_string())),
        }
    }
}

fn sentence(message: &str) -> String {
    let mut chars = message.chars();
    let first: String = chars.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    let text = format!("{first}{}", chars.as_str());
    if text.ends_with('.') { text } else { format!("{text}.") }
}

impl Launcher {
    pub async fn skins_list(&self) -> Vec<SkinAsset> {
        self.skin_library.list().await
    }

    pub async fn skins_import(&self, bytes: &[u8], name: &str, model: Option<SkinModel>) -> Result<SkinAsset, LauncherError> {
        Ok(self.skin_library.import(bytes, name, model).await?)
    }

    /// PNG como data URL para previsualizar o editar en la interfaz.
    pub async fn skins_read(&self, id: &str) -> Result<String, LauncherError> {
        Ok(self.skin_library.read_data_url(id).await?)
    }

    pub async fn skins_rename(&self, id: &str, name: &str) -> Result<SkinAsset, LauncherError> {
        Ok(self.skin_library.rename(id, name).await?)
    }

    pub async fn skins_set_model(&self, id: &str, model: SkinModel) -> Result<SkinAsset, LauncherError> {
        Ok(self.skin_library.set_model(id, model).await?)
    }

    /// Reemplaza los pixeles de una skin (editor integrado).
    pub async fn skins_replace_pixels(&self, id: &str, png: &[u8]) -> Result<SkinAsset, LauncherError> {
        Ok(self.skin_library.replace_pixels(id, png).await?)
    }

    /// Borra la skin y la desvincula de los perfiles que la usaban.
    pub async fn skins_remove(&self, id: &str) -> Result<(), LauncherError> {
        self.skin_library.remove(id).await?;
        for mut account in self.accounts.list().await? {
            if let ProviderAccountData::Local(data) = &mut account.provider_data {
                if data.skin_id.as_deref() == Some(id) {
                    data.skin_id = None;
                    self.accounts.save(&account).await?;
                }
            }
        }
        Ok(())
    }

    /// Copia el PNG a `dest` (elegido por el usuario).
    pub async fn skins_export(&self, id: &str, dest: &Path) -> Result<(), LauncherError> {
        let png = self.skin_library.read_png(id).await?;
        tokio::fs::write(dest, png).await.map_err(|_| LauncherError::Storage)
    }

    /// Asocia (o quita) la skin de un perfil local. Los perfiles Microsoft usan su skin oficial.
    pub async fn accounts_set_skin(&self, account_id: &str, skin_id: Option<&str>) -> Result<(), LauncherError> {
        let mut account = self.accounts.get(account_id).await?.ok_or(LauncherError::AccountNotFound)?;
        if account.provider != IdentityProviderKind::Local {
            return Err(LauncherError::InvalidInput(
                "La skin de una cuenta Microsoft se gestiona desde Minecraft, no desde aqui.".into(),
            ));
        }
        if let Some(id) = skin_id {
            if self.skin_library.get(id).await.is_none() {
                return Err(LauncherError::InvalidInput("No se encontro esa skin.".into()));
            }
        }
        if let ProviderAccountData::Local(data) = &mut account.provider_data {
            data.skin_id = skin_id.map(str::to_string);
        }
        self.accounts.save(&account).await?;
        Ok(())
    }
}
