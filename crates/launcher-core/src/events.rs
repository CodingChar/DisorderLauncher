use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgressEvent {
    pub instance_id: String,
    pub phase: &'static str,
    pub completed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum LaunchState {
    Preparing,
    Installing,
    PreparingJava,
    Starting,
    Running { pid: u32 },
    Exited { code: Option<i32> },
    Failed { message: String, kind: crate::ErrorKind },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStateEvent {
    pub instance_id: String,
    #[serde(flatten)]
    pub state: LaunchState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOutputEvent {
    pub instance_id: String,
    pub line: String,
}

/// Progreso del inicio de sesion con Microsoft (`auth://state`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum AuthStateEvent {
    /// El navegador del sistema esta abierto esperando al usuario.
    Authorizing,
    #[serde(rename_all = "camelCase")]
    Connected { account_id: String, name: String },
    Cancelled,
    Failed { message: String },
}

/// Destino de los eventos del launcher. La app de escritorio lo implementa emitiendo
/// eventos Tauri; los tests usan un recolector.
pub trait LauncherEvents: Send + Sync {
    fn install_progress(&self, event: InstallProgressEvent);
    fn launch_state(&self, event: LaunchStateEvent);
    fn game_output(&self, stream: OutputStream, event: GameOutputEvent);
    /// Estado del inicio de sesion con Microsoft.
    fn auth_state(&self, _event: AuthStateEvent) {}
    /// Avance de una reparacion (mismo formato que el de instalacion).
    fn repair_progress(&self, _event: InstallProgressEvent) {}
}

/// Resultado de reparar una instancia, pensado para mostrarse al usuario.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepairSummary {
    /// Archivos con problemas antes de reparar.
    pub found: usize,
    pub fixed: usize,
    /// Problemas que siguen sin resolverse.
    pub remaining: usize,
}

pub struct NoopEvents;

impl LauncherEvents for NoopEvents {
    fn install_progress(&self, _: InstallProgressEvent) {}
    fn launch_state(&self, _: LaunchStateEvent) {}
    fn game_output(&self, _: OutputStream, _: GameOutputEvent) {}
}
