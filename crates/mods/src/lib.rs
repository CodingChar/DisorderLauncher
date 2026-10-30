//! Gestion de mods de una instancia: archivos locales, metadatos de Fabric, registro de
//! origen (Modrinth), resolucion de dependencias, actualizaciones y rollback.
//! No conoce identidad ni cuentas (spec 07).

mod conflicts;
mod error;
mod local;
mod lock;
mod metadata;
mod modrinth;
mod resolve;
mod service;

pub use conflicts::*;
pub use error::*;
pub use local::*;
pub use lock::*;
pub use metadata::*;
pub use modrinth::*;
pub use resolve::*;
pub use service::*;
