use launcher_core::{
  GameOutputEvent, InstallProgressEvent, LaunchStateEvent, LauncherEvents, OutputStream,
};
use tauri::{AppHandle, Emitter};

pub const INSTALL_PROGRESS: &str = "install://progress";
pub const LAUNCH_STATE: &str = "launch://state";
pub const LAUNCH_STDOUT: &str = "launch://stdout";
pub const LAUNCH_STDERR: &str = "launch://stderr";

/// Reenvia los eventos del launcher al frontend como eventos Tauri.
pub struct TauriEvents {
  app: AppHandle,
}

impl TauriEvents {
  pub fn new(app: AppHandle) -> Self {
    Self { app }
  }
}

impl LauncherEvents for TauriEvents {
  fn install_progress(&self, event: InstallProgressEvent) {
    let _ = self.app.emit(INSTALL_PROGRESS, event);
  }

  fn launch_state(&self, event: LaunchStateEvent) {
    let _ = self.app.emit(LAUNCH_STATE, event);
  }

  fn game_output(&self, stream: OutputStream, event: GameOutputEvent) {
    let name = match stream {
      OutputStream::Stdout => LAUNCH_STDOUT,
      OutputStream::Stderr => LAUNCH_STDERR,
    };
    let _ = self.app.emit(name, event);
  }
}
