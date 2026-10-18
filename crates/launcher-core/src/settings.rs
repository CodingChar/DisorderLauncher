use crate::LauncherError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// Cuantas descargas simultaneas se permiten (spec 02 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadProfile {
    Eco,
    Normal,
    Performance,
}

impl DownloadProfile {
    pub fn concurrency(self) -> usize {
        match self {
            Self::Eco => 3,
            Self::Normal => 6,
            Self::Performance => 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub theme: Theme,
    pub download_profile: DownloadProfile,
    /// Tema personalizado elegido; `None` usa el tema incluido segun `theme`.
    pub theme_id: Option<String>,
    /// Id de la app registrada en Azure (opcional, avanzado). Sin el, Microsoft queda deshabilitado.
    pub microsoft_client_id: Option<String>,
    /// Carpeta de Java usada por las instancias con Java "automatico" cuando es compatible.
    pub java_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            theme: Theme::System,
            download_profile: DownloadProfile::Normal,
            theme_id: None,
            microsoft_client_id: None,
            java_path: None,
        }
    }
}

impl Settings {
    /// Normaliza y valida los ajustes recibidos de la interfaz.
    pub fn validated(mut self) -> Result<Self, LauncherError> {
        self.schema_version = SCHEMA_VERSION;
        self.java_path = self.java_path.filter(|p| !p.as_os_str().is_empty());
        self.microsoft_client_id = match self.microsoft_client_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => Some(identity_microsoft::validate_client_id(id).map_err(|_| {
                LauncherError::InvalidInput("El identificador de la aplicacion de Microsoft no es valido.".into())
            })?),
            None => None,
        };
        if let Some(path) = &self.java_path {
            if !java_runtime::java_executable(path).is_file() {
                return Err(LauncherError::InvalidInput("No se encontro Java en esa carpeta.".into()));
            }
        }
        Ok(self)
    }
}

/// Ajustes mas datos de solo lectura que la interfaz muestra en la pantalla de ajustes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub settings: Settings,
    pub data_dir: String,
}

/// Lee los ajustes. Un archivo ausente, ilegible o de un esquema futuro devuelve los
/// valores por defecto sin tocar el archivo: nunca se pierde configuracion por un fallo de lectura.
pub async fn load_settings(path: &Path) -> Settings {
    let Ok(data) = tokio::fs::read_to_string(path).await else {
        return Settings::default();
    };
    match serde_json::from_str::<Settings>(&data) {
        Ok(s) if s.schema_version <= SCHEMA_VERSION => s,
        Ok(_) => {
            tracing::warn!("settings.json es de una version mas nueva; se usan valores por defecto");
            Settings::default()
        }
        Err(e) => {
            tracing::warn!(error = %e, "settings.json ilegible; se usan valores por defecto (copia en settings.json.corrupt)");
            let _ = tokio::fs::write(path.with_extension("json.corrupt"), &data).await;
            Settings::default()
        }
    }
}

pub async fn save_settings(path: &Path, settings: &Settings) -> Result<(), LauncherError> {
    let write = async {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = path.with_extension("json.part");
        tokio::fs::write(&part, serde_json::to_string_pretty(settings)?).await?;
        tokio::fs::rename(&part, path).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    write.await.map_err(|e| {
        tracing::error!(error = %e, "no se pudieron guardar los ajustes");
        LauncherError::Storage
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_map_to_spec_concurrency() {
        assert_eq!(DownloadProfile::Eco.concurrency(), 3);
        assert_eq!(DownloadProfile::Normal.concurrency(), 6);
        assert_eq!(DownloadProfile::Performance.concurrency(), 10);
    }

    #[tokio::test]
    async fn roundtrip_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        assert_eq!(load_settings(&path).await, Settings::default());

        let custom = Settings { theme: Theme::Dark, download_profile: DownloadProfile::Eco, ..Settings::default() };
        save_settings(&path, &custom).await.unwrap();
        assert_eq!(load_settings(&path).await, custom);

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"downloadProfile\": \"eco\""));
    }

    #[tokio::test]
    async fn corrupt_or_future_files_fall_back_to_defaults_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        std::fs::write(&path, "{ nope").unwrap();
        assert_eq!(load_settings(&path).await, Settings::default());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ nope");

        std::fs::write(&path, r#"{"schemaVersion": 99, "theme": "dark"}"#).unwrap();
        assert_eq!(load_settings(&path).await, Settings::default());
    }

    #[tokio::test]
    async fn partial_files_use_defaults_for_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"theme": "light"}"#).unwrap();
        let s = load_settings(&path).await;
        assert_eq!(s.theme, Theme::Light);
        assert_eq!(s.download_profile, DownloadProfile::Normal);
    }

    #[test]
    fn java_path_must_contain_java() {
        let dir = tempfile::tempdir().unwrap();
        let bad = Settings { java_path: Some(dir.path().to_path_buf()), ..Settings::default() };
        assert!(bad.validated().is_err());

        let empty = Settings { java_path: Some(PathBuf::new()), ..Settings::default() };
        assert_eq!(empty.validated().unwrap().java_path, None);

        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join(if cfg!(windows) { "java.exe" } else { "java" }), b"").unwrap();
        let ok = Settings { java_path: Some(dir.path().to_path_buf()), ..Settings::default() };
        assert!(ok.validated().is_ok());
    }
}
