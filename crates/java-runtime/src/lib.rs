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

#[derive(Debug, Clone)]
pub struct JavaRequirement {
    pub major_version: u32,
}
