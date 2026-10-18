use crate::events::TauriEvents;
use launcher_core::{Launcher, LauncherPaths};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

/// Estado global de la app: el orquestador del launcher. Los secretos viven dentro
/// de `launcher-core`; el frontend solo recibe datos ya resueltos y sin tokens.
pub struct AppState {
  pub launcher: Arc<Launcher>,
  /// Ultima carpeta de Minecraft elegida con el selector: el frontend solo puede usar esa o la habitual,
  /// nunca una ruta arbitraria.
  pub picked_minecraft_dir: Mutex<Option<PathBuf>>,
}

impl AppState {
  pub fn new(data_dir: PathBuf, app: AppHandle) -> Self {
    let launcher = Launcher::new(LauncherPaths::new(data_dir), Arc::new(TauriEvents::new(app)));
    Self { launcher: Arc::new(launcher), picked_minecraft_dir: Mutex::new(None) }
  }
}
