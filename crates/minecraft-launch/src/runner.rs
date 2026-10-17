use crate::{LaunchError, LaunchPlan};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Variables del entorno del launcher que el juego si necesita. El resto (tokens,
/// claves de API, etc.) no se hereda.
const INHERITED_ENV: &[&str] = &[
    "PATH", "PATHEXT", "SystemRoot", "SYSTEMDRIVE", "windir", "ComSpec", "TEMP", "TMP",
    "USERPROFILE", "APPDATA", "LOCALAPPDATA", "ProgramData", "ProgramFiles",
    "ProgramFiles(x86)", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE", "OS",
    "HOME", "USER", "LANG", "LC_ALL", "DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY",
    "XDG_RUNTIME_DIR", "XDG_SESSION_TYPE", "XDG_DATA_DIRS",
];

const CHANNEL_CAPACITY: usize = 512;
const MIN_REDACTABLE_SECRET_LEN: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchEvent {
    Started { pid: u32 },
    Stdout(String),
    Stderr(String),
    Exited { code: Option<i32> },
}

/// Descripcion de un proceso a lanzar: ejecutable y argv, sin shell de por medio.
#[derive(Debug, Clone)]
pub struct RunSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    pub secrets: Vec<String>,
}

impl RunSpec {
    pub fn from_plan(plan: &LaunchPlan) -> Result<Self, LaunchError> {
        Ok(Self {
            program: plan.java_path.clone(),
            args: plan.command_args()?,
            working_directory: plan.working_directory.clone(),
            environment: plan.environment.clone(),
            secrets: plan.secrets().to_vec(),
        })
    }
}

pub struct ProcessRunner;

impl ProcessRunner {
    /// Valida el plan, crea el directorio de juego y lanza el proceso.
    pub async fn launch(plan: &LaunchPlan) -> Result<GameHandle, LaunchError> {
        plan.validate()?;
        Self::spawn(RunSpec::from_plan(plan)?).await
    }

    pub async fn spawn(spec: RunSpec) -> Result<GameHandle, LaunchError> {
        tokio::fs::create_dir_all(&spec.working_directory)
            .await
            .map_err(LaunchError::WorkingDirectory)?;

        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .current_dir(&spec.working_directory)
            .env_clear()
            .envs(inherited_env())
            .envs(&spec.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW: evita abrir una consola junto al juego.
            command.creation_flags(0x0800_0000);
        }

        let mut child = command.spawn().map_err(LaunchError::Spawn)?;
        let pid = child.id().unwrap_or_default();
        tracing::info!(pid, "proceso de Minecraft iniciado");

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (kill_tx, mut kill_rx) = mpsc::channel::<()>(1);

        let secrets: Vec<String> = spec
            .secrets
            .into_iter()
            .filter(|s| s.len() >= MIN_REDACTABLE_SECRET_LEN)
            .collect();

        let _ = tx.send(LaunchEvent::Started { pid }).await;

        let out_task = stdout.map(|s| {
            tokio::spawn(pump_lines(s, tx.clone(), secrets.clone(), LaunchEvent::Stdout))
        });
        let err_task = stderr.map(|s| {
            tokio::spawn(pump_lines(s, tx.clone(), secrets.clone(), LaunchEvent::Stderr))
        });

        tokio::spawn(async move {
            let status = tokio::select! {
                status = child.wait() => status,
                _ = kill_rx.recv() => {
                    let _ = child.kill().await;
                    child.wait().await
                }
            };

            // Esperar a que se vacien los pipes para que `Exited` sea el ultimo evento.
            for task in [out_task, err_task].into_iter().flatten() {
                let _ = task.await;
            }

            let code = status.ok().and_then(|s| s.code());
            tracing::info!(?code, "proceso de Minecraft terminado");
            let _ = tx.send(LaunchEvent::Exited { code }).await;
        });

        Ok(GameHandle { pid, events: rx, kill_tx })
    }
}

pub struct GameHandle {
    pid: u32,
    events: mpsc::Receiver<LaunchEvent>,
    kill_tx: mpsc::Sender<()>,
}

impl GameHandle {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Siguiente evento; `None` cuando el proceso termino y ya se leyo `Exited`.
    pub async fn next_event(&mut self) -> Option<LaunchEvent> {
        self.events.recv().await
    }

    /// Pide el cierre del proceso. Seguro de llamar varias veces.
    pub async fn kill(&self) {
        let _ = self.kill_tx.try_send(());
    }

    /// Consume los eventos hasta la salida y devuelve el codigo de salida.
    pub async fn wait(mut self) -> Option<i32> {
        while let Some(event) = self.events.recv().await {
            if let LaunchEvent::Exited { code } = event {
                return code;
            }
        }
        None
    }
}

fn inherited_env() -> Vec<(OsString, OsString)> {
    INHERITED_ENV
        .iter()
        .filter_map(|key| std::env::var_os(key).map(|v| (OsString::from(key), v)))
        .collect()
}

fn redact(line: &str, secrets: &[String]) -> String {
    let mut out = line.to_string();
    for secret in secrets {
        if out.contains(secret.as_str()) {
            out = out.replace(secret.as_str(), "<redacted>");
        }
    }
    out
}

async fn pump_lines<R>(
    reader: R,
    tx: mpsc::Sender<LaunchEvent>,
    secrets: Vec<String>,
    wrap: fn(String) -> LaunchEvent,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buf);
                let line = redact(text.trim_end_matches(['\r', '\n']), &secrets);
                if tx.send(wrap(line)).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Solo para tests: un programa del sistema que imprime en stdout/stderr y sale.
    fn shell_spec(script: &str, secrets: Vec<String>) -> RunSpec {
        let (program, args) = if cfg!(windows) {
            ("cmd.exe", vec!["/C".to_string(), script.to_string()])
        } else {
            ("sh", vec!["-c".to_string(), script.to_string()])
        };
        RunSpec {
            program: program.into(),
            args,
            working_directory: std::env::temp_dir(),
            environment: HashMap::new(),
            secrets,
        }
    }

    async fn collect(mut handle: GameHandle) -> Vec<LaunchEvent> {
        let mut events = Vec::new();
        while let Some(e) = handle.next_event().await {
            events.push(e);
        }
        events
    }

    #[tokio::test]
    async fn streams_output_and_exit_code() {
        let script = if cfg!(windows) {
            "echo hola& echo problema>&2& exit 3"
        } else {
            "echo hola; echo problema >&2; exit 3"
        };
        let spec = shell_spec(script, vec![]);
        let events = collect(ProcessRunner::spawn(spec).await.unwrap()).await;

        assert!(matches!(events.first(), Some(LaunchEvent::Started { .. })));
        assert!(events.contains(&LaunchEvent::Stdout("hola".into())));
        assert!(events.contains(&LaunchEvent::Stderr("problema".into())));
        assert_eq!(events.last(), Some(&LaunchEvent::Exited { code: Some(3) }));
    }

    #[tokio::test]
    async fn redacts_secrets_in_output() {
        let secret = "super-secret-token".to_string();
        let script = format!("echo token={secret}");
        let events = collect(ProcessRunner::spawn(shell_spec(&script, vec![secret])).await.unwrap()).await;

        assert!(events.contains(&LaunchEvent::Stdout("token=<redacted>".into())));
    }

    #[tokio::test]
    async fn does_not_inherit_unlisted_environment() {
        std::env::set_var("DISORDER_TEST_SECRET", "leak");
        let script = if cfg!(windows) {
            "echo [%DISORDER_TEST_SECRET%]"
        } else {
            "echo [$DISORDER_TEST_SECRET]"
        };
        let events = collect(ProcessRunner::spawn(shell_spec(script, vec![])).await.unwrap()).await;

        let leaked = events.iter().any(|e| matches!(e, LaunchEvent::Stdout(l) if l.contains("leak")));
        assert!(!leaked);
    }

    #[tokio::test]
    async fn missing_program_is_spawn_error() {
        let mut spec = shell_spec("", vec![]);
        spec.program = "definitivamente-no-existe-xyz".into();
        assert!(matches!(ProcessRunner::spawn(spec).await, Err(LaunchError::Spawn(_))));
    }

    #[tokio::test]
    async fn kill_terminates_long_running_process() {
        let script = if cfg!(windows) { "ping -n 30 127.0.0.1 >nul" } else { "sleep 30" };
        let handle = ProcessRunner::spawn(shell_spec(script, vec![])).await.unwrap();
        handle.kill().await;

        let code = tokio::time::timeout(std::time::Duration::from_secs(10), handle.wait())
            .await
            .expect("el proceso debio terminar tras kill");
        assert_ne!(code, Some(0));
    }
}
