//! Proveedor de identidad Microsoft: Authorization Code + PKCE con navegador del sistema,
//! cadena Xbox -> XSTS -> Minecraft Services y verificacion de propiedad (spec 04).
//!
//! Cliente publico: no hay secreto en el binario. Sin `client_id` configurado el proveedor
//! queda deshabilitado y responde `MicrosoftNotConfigured`; nunca se simula una sesion.

mod auth;
mod config;
mod fake;
mod loopback;
mod pkce;
mod provider;
mod secret;

pub use auth::*;
pub use config::*;
pub use fake::*;
pub use loopback::*;
pub use pkce::*;
pub use provider::*;
pub use secret::*;
