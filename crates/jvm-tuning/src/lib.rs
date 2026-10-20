//! Ajustes de rendimiento de la JVM: catalogo respaldado por fuentes, validacion de argumentos propios,
//! resolucion segun Java y equipo, y comprobacion real contra el Java que se va a usar.
//!
//! La logica de decision es pura (sin E/S); solo `probe` ejecuta Java.

mod args;
mod catalog;
mod probe;
mod resolve;

pub use args::{key, parse as parse_args, selected_gc, ArgError, UnknownGc, MAX_ARGS, MAX_ARG_LEN};
pub use catalog::{find, Env, Gc, Risk, Tweak, CATALOG};
pub use probe::{final_value, keep_working, probe};
pub use resolve::{resolve, unavailable_reason, Group, Resolved, Skipped};

#[cfg(test)]
mod java_tests;
