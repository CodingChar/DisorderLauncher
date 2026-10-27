//! Almacen comun de artefactos inmutables identificados por el SHA-512 de su contenido.
//!
//! El almacen solo guarda objetos (`sha512/<2 hex>/<128 hex>`); quien usa cada objeto lo dice el
//! indice de cada instancia (`game/.disorder/artifacts.json`). Minecraft sigue viendo archivos
//! normales en `mods/`: se materializan como hardlink o, si el sistema de archivos no lo permite, copia.

mod adopt;
mod error;
mod hash;
mod index;
mod store;

pub use adopt::{adopt_instance, AdoptReport, STORE_DIRS};
pub use error::ArtifactError;
pub use hash::{hash_file, hash_reader, is_valid_hash};
pub use index::{ArtifactEntry, ArtifactIndex, ArtifactSource, INDEX_FILE};
pub use store::{ArtifactStore, Ingested, Method, StoredObject};
