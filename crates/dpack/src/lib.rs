//! Formato portable de instancia: un `.7z` estandar con `dpack-manifest.json` y el contenido del pack.
//!
//! El paquete es autocontenido (los mods viajan dentro) y no depende del almacen local ni de ningun
//! proveedor. Esta capa solo conoce el formato; la instancia, el almacen y la red los pone el launcher.

mod error;
mod export;
mod import;
mod manifest;
mod path;

pub use error::DpackError;
pub use export::{export, ExportReport, ExportRequest};
pub use import::{extract, read_manifest, ExtractPlan, Limits};
pub use manifest::{EntrySource, LoaderInfo, Manifest, ManifestEntry, MinecraftInfo, PackInfo, FORMAT_VERSION, MANIFEST_NAME};
pub use path::validate_entry_path;
