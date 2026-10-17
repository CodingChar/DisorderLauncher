use crate::events::TauriEvents;
use launcher_core::{Launcher, LauncherPaths};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;

/// Estado global de la app: el orquestador del launcher. Los secretos viven dentro
/// de `launcher-core`; el frontend solo recibe datos ya resueltos y sin tokens.
pub struct AppState {
  pub launcher: Arc<Launcher>,
}

impl AppState {
  pub fn new(data_dir: PathBuf, app: AppHandle) -> Self {
    let launcher = Launcher::new(LauncherPaths::new(data_dir), Arc::new(TauriEvents::new(app)));
    Self { launcher: Arc::new(launcher) }
  }
}
