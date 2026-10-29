//! Operaciones de archivos que aguantan lo que pasa de verdad en un equipo de usuario.
//!
//! En Windows un archivo abierto por otro programa (el antivirus, el indexador, Java con su registro,
//! la ventana de errores de un mod) no se puede borrar ni reemplazar mientras dure el bloqueo: el sistema
//! responde con "sharing violation" (error 32) o "acceso denegado". La mayoria de bloqueos duran
//! milisegundos; otros duran hasta que se cierra el programa. Las funciones de este crate:
//!
//! - reintentan con espera creciente los fallos transitorios (renombrar, escribir, borrar);
//! - al borrar un arbol, quitan todo lo que se puede y solo dejan lo bloqueado, en vez de parar en el
//!   primer archivo (con `std::fs::remove_dir_all` un solo archivo bloqueado deja casi todo en su sitio).
//!
//! Todo el launcher debe usar estas funciones para escribir y mover sus archivos.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Esperas entre reintentos (~3 s en total): cubre antivirus e indexadores sin hacer esperar al usuario.
const RETRY_DELAYS_MS: [u64; 7] = [25, 50, 100, 200, 400, 800, 1200];

/// `true` si el error suele desaparecer solo: archivo en uso, acceso denegado momentaneo o bloqueo.
pub fn is_transient(error: &io::Error) -> bool {
    if matches!(error.kind(), io::ErrorKind::PermissionDenied | io::ErrorKind::ResourceBusy | io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock) {
        return true;
    }
    // Windows: 5 acceso denegado, 32 violacion de uso compartido, 33 bloqueo, 145 carpeta no vacia
    // (aun con borrados pendientes), 1224 archivo con seccion mapeada, 303 borrado pendiente.
    matches!(error.raw_os_error(), Some(5 | 32 | 33 | 145 | 303 | 1224))
}

/// `true` si el error indica que otro programa tiene el archivo abierto.
pub fn is_in_use(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33 | 1224)) || error.kind() == io::ErrorKind::ResourceBusy
}

/// Reintenta `op` mientras falle con un error transitorio.
fn retry_blocking<T>(mut op: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    let mut delays = RETRY_DELAYS_MS.iter();
    loop {
        match op() {
            Err(e) if is_transient(&e) => match delays.next() {
                Some(ms) => std::thread::sleep(Duration::from_millis(*ms)),
                None => return Err(e),
            },
            other => return other,
        }
    }
}

/// Renombra con reintentos (sincrono). Sustituye el destino si existe.
pub fn rename_blocking(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let (from, to) = (from.as_ref(), to.as_ref());
    retry_blocking(|| std::fs::rename(from, to))
}

/// Renombra con reintentos. Sustituye el destino si existe.
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let (from, to) = (from.as_ref().to_path_buf(), to.as_ref().to_path_buf());
    blocking(move || rename_blocking(from, to)).await
}

/// Borra un archivo con reintentos. Que no exista no es un error.
pub fn remove_file_blocking(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    match retry_blocking(|| remove_one_file(path)) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Borra un archivo con reintentos. Que no exista no es un error.
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    blocking(move || remove_file_blocking(path)).await
}

fn remove_one_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
            // Un archivo de solo lectura no se borra en Windows: se le quita la marca y se reintenta.
            clear_readonly(path);
            std::fs::remove_file(path)
        }
        other => other,
    }
}

fn clear_readonly(path: &Path) {
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        let mut permissions = meta.permissions();
        if permissions.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            permissions.set_readonly(false);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
}

/// Escribe `bytes` en `path` de forma atomica: archivo temporal en la misma carpeta y renombrado, con
/// reintentos si el antivirus o el indexador tienen el temporal o el destino bloqueado un instante.
pub fn write_atomic_blocking(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();
    let part = part_path(path);
    let written = retry_blocking(|| std::fs::write(&part, bytes.as_ref()));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    rename_blocking(&part, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&part);
    })
}

/// Version asincrona de [`write_atomic_blocking`].
pub async fn write_atomic(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    let bytes = bytes.as_ref().to_vec();
    blocking(move || write_atomic_blocking(path, bytes)).await
}

/// Temporal unico junto al destino: dos escrituras a la vez no se pisan.
fn part_path(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "archivo".into());
    path.with_file_name(format!("{name}.{}-{}.part", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)))
}

/// Resultado de borrar un arbol de carpetas.
#[derive(Debug)]
pub enum RemoveOutcome {
    /// Ya no queda nada (o nunca existio).
    Removed,
    /// Quedan entradas que otro programa mantiene abiertas: se borraran cuando se liberen.
    Partial { remaining: usize, error: io::Error },
}

impl RemoveOutcome {
    pub fn is_removed(&self) -> bool {
        matches!(self, Self::Removed)
    }
}

/// Borra `path` entero. Quita todo lo que se puede aunque algo falle y reintenta lo bloqueado con espera
/// creciente. No sigue enlaces simbolicos ni uniones: borra el enlace, no su destino.
pub fn remove_tree_blocking(path: impl AsRef<Path>) -> io::Result<RemoveOutcome> {
    let path = path.as_ref();
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(RemoveOutcome::Removed),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    let mut delays = RETRY_DELAYS_MS.iter();
    loop {
        let mut failure = None;
        let remaining = sweep(path, &mut failure);
        match failure {
            None if remaining == 0 => return Ok(RemoveOutcome::Removed),
            Some(error) if !is_transient(&error) => return Ok(RemoveOutcome::Partial { remaining, error }),
            failure => match delays.next() {
                Some(ms) => std::thread::sleep(Duration::from_millis(*ms)),
                None => {
                    let error = failure.unwrap_or_else(|| io::Error::other("quedan archivos"));
                    return Ok(RemoveOutcome::Partial { remaining, error });
                }
            },
        }
    }
}

/// Version asincrona de [`remove_tree_blocking`].
pub async fn remove_tree(path: impl AsRef<Path>) -> io::Result<RemoveOutcome> {
    let path = path.as_ref().to_path_buf();
    blocking(move || remove_tree_blocking(path)).await
}

/// Una pasada: borra lo que puede y devuelve cuantas entradas quedan. Guarda el primer error.
fn sweep(path: &Path, first_error: &mut Option<io::Error>) -> usize {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    let mut note = |e: io::Error| {
        if first_error.is_none() {
            *first_error = Some(e);
        }
    };
    if meta.is_dir() && !meta.file_type().is_symlink() {
        let mut remaining = 0;
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    remaining += sweep(&entry.path(), first_error);
                }
            }
            Err(e) => {
                note(e);
                remaining += 1;
            }
        }
        match std::fs::remove_dir(path) {
            Ok(()) => 0,
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => {
                if first_error.is_none() {
                    *first_error = Some(e);
                }
                remaining + 1
            }
        }
    } else {
        match remove_one_file(path) {
            Ok(()) => 0,
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => {
                note(e);
                1
            }
        }
    }
}

async fn blocking<T: Send + 'static>(op: impl FnOnce() -> io::Result<T> + Send + 'static) -> io::Result<T> {
    tokio::task::spawn_blocking(op).await.map_err(|e| io::Error::other(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_paths_are_not_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remove_file_blocking(dir.path().join("no-existe")).is_ok());
        assert!(remove_tree_blocking(dir.path().join("no-existe")).unwrap().is_removed());
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temporaries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("datos.json");
        write_atomic_blocking(&path, b"uno").unwrap();
        write_atomic_blocking(&path, b"dos").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"dos");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn rename_replaces_an_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), b"nuevo").unwrap();
        std::fs::write(dir.path().join("b"), b"viejo").unwrap();
        rename(dir.path().join("a"), dir.path().join("b")).await.unwrap();
        assert_eq!(std::fs::read(dir.path().join("b")).unwrap(), b"nuevo");
    }

    #[test]
    fn remove_tree_deletes_nested_content_including_readonly_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("instancia");
        std::fs::create_dir_all(root.join("game/mods")).unwrap();
        std::fs::write(root.join("game/mods/a.jar"), b"x").unwrap();
        let readonly = root.join("game/options.txt");
        std::fs::write(&readonly, b"x").unwrap();
        let mut permissions = std::fs::metadata(&readonly).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&readonly, permissions).unwrap();
        assert!(remove_tree_blocking(&root).unwrap().is_removed());
        assert!(!root.exists());
    }

    /// Un archivo que otro programa mantiene abierto (como Java con su registro) no debe impedir borrar
    /// todo lo demas, y se puede terminar de borrar cuando se libera.
    #[cfg(windows)]
    #[test]
    fn a_locked_file_only_blocks_itself() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("instancia");
        std::fs::create_dir_all(root.join("game/logs")).unwrap();
        std::fs::create_dir_all(root.join("game/mods")).unwrap();
        std::fs::write(root.join("game/logs/latest.log"), b"x").unwrap();
        std::fs::write(root.join("game/mods/a.jar"), b"x").unwrap();
        std::fs::write(root.join("game/options.txt"), b"x").unwrap();
        // Comparte lectura y escritura pero no borrado, igual que Java.
        let held = std::fs::OpenOptions::new().write(true).share_mode(3).open(root.join("game/logs/latest.log")).unwrap();

        let outcome = remove_tree_blocking(&root).unwrap();
        match outcome {
            RemoveOutcome::Partial { remaining, error } => {
                assert!(remaining >= 1 && is_in_use(&error), "{remaining} {error:?}");
            }
            RemoveOutcome::Removed => panic!("el archivo en uso no puede haberse borrado"),
        }
        assert!(!root.join("game/mods").exists(), "lo demas si se borro");
        assert!(!root.join("game/options.txt").exists());

        drop(held);
        assert!(remove_tree_blocking(&root).unwrap().is_removed());
        assert!(!root.exists());
    }

    #[test]
    fn classifies_errors() {
        assert!(is_transient(&io::Error::from_raw_os_error(32)));
        assert!(is_in_use(&io::Error::from_raw_os_error(32)));
        assert!(!is_transient(&io::Error::from(io::ErrorKind::NotFound)));
        assert!(!is_in_use(&io::Error::from(io::ErrorKind::PermissionDenied)));
    }
}
