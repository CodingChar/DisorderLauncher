//! Modpacks en formato `.mrpack` de Modrinth: lectura validada, importacion y exportacion.
//! El formato interno es declarativo y reproducible: archivos con hash + overrides controlados.

mod error;
mod export;
mod import;
mod index;

pub use error::*;
pub use export::*;
pub use import::*;
pub use index::*;
