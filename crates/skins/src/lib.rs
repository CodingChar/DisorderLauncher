//! Biblioteca de skins locales. Una skin es un recurso del launcher (`LocalSkinAsset`),
//! distinto de la skin oficial de un perfil Microsoft: nunca se mezclan (spec 10).

mod adapt;
mod error;
mod library;
mod png_check;

pub use adapt::*;
pub use error::*;
pub use library::*;
pub use png_check::*;
