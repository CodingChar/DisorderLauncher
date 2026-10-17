mod error;
mod detect;
mod adoptium;
mod manager;

pub use error::*;
pub use detect::*;
pub use adoptium::*;
pub use manager::*;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub major: u32,
    pub managed: bool,
}

impl JavaInstallation {
    /// Ruta al ejecutable `java` dentro del home de esta instalacion.
    pub fn executable(&self) -> PathBuf {
        java_executable(&self.path)
    }
}

#[derive(Debug, Clone)]
pub struct JavaRequirement {
    pub major_version: u32,
}
