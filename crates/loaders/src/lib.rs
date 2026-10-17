//! Proveedores de loaders (Fabric, ...). Un loader se aplica sobre una version vanilla ya
//! instalada y produce un `LaunchOverlay`. Nunca recibe tokens de cuenta (spec 07).

mod error;
mod fabric;
mod trust;

pub use error::*;
pub use fabric::*;
pub use trust::*;

use instance_manager::LoaderKind;
use minecraft_launch::LaunchOverlay;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

pub struct LoaderInstallContext<'a> {
    /// Raiz compartida con `versions/` y `libraries/`.
    pub data_dir: &'a Path,
    pub minecraft_version: &'a str,
    pub loader_version: &'a str,
    pub concurrency: usize,
    /// `(completados, total)` de la descarga de librerias.
    pub progress: Arc<dyn Fn(usize, usize) + Send + Sync>,
}

#[derive(Debug, Clone)]
pub struct InstalledLoader {
    /// Identificador del perfil guardado en `versions/<id>/<id>.json`.
    pub profile_id: String,
    pub overlay: LaunchOverlay,
}

#[async_trait::async_trait]
pub trait LoaderProvider: Send + Sync {
    fn kind(&self) -> LoaderKind;

    /// Versiones del loader compatibles con `minecraft_version`, la mas reciente primero.
    async fn versions(&self, minecraft_version: &str) -> Result<Vec<LoaderVersion>, LoaderError>;

    /// Descarga el perfil y sus librerias y devuelve el overlay de lanzamiento.
    async fn install(&self, ctx: &LoaderInstallContext<'_>) -> Result<InstalledLoader, LoaderError>;

    /// Carga una instalacion previa solo desde disco (sin red). `None` si falta algo.
    async fn load_installed(
        &self,
        data_dir: &Path,
        minecraft_version: &str,
        loader_version: &str,
    ) -> Result<Option<InstalledLoader>, LoaderError>;
}
