//! Temas: la validacion vive en el crate `themes`; aqui solo se traducen errores.

use crate::{Launcher, LauncherError};
use themes::{Theme, ThemeEntry, ThemeError};

impl From<ThemeError> for LauncherError {
    fn from(e: ThemeError) -> Self {
        tracing::warn!(error = %e, "tema rechazado");
        match e {
            ThemeError::Io => Self::Storage,
            ThemeError::NotFound => Self::InvalidInput("No se encontro ese tema.".into()),
            ThemeError::Builtin => Self::InvalidInput("Los temas incluidos no se pueden quitar.".into()),
            ThemeError::Duplicate => Self::InvalidInput("Ya tienes un tema con ese identificador.".into()),
            other => Self::InvalidInput(format!("Ese tema no es valido: {other}.")),
        }
    }
}

impl Launcher {
    pub async fn themes_list(&self) -> Vec<ThemeEntry> {
        self.theme_store.list().await
    }

    pub async fn themes_import(&self, json: &[u8]) -> Result<Theme, LauncherError> {
        Ok(self.theme_store.import(json).await?)
    }

    /// Quita un tema personalizado; si estaba en uso se vuelve al tema incluido.
    pub async fn themes_remove(&self, id: &str) -> Result<(), LauncherError> {
        self.theme_store.remove(id).await?;
        let mut current = self.settings();
        if current.theme_id.as_deref() == Some(id) {
            current.theme_id = None;
            self.settings_update(current).await?;
        }
        Ok(())
    }
}
