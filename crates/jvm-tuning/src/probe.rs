//! Comprobacion real: se arranca el propio Java con las banderas (`java <banderas> -version`). Si la JVM las
//! acepta, el juego tambien las aceptara; si no, se descartan solo las que fallan y el juego sigue arrancando.

use crate::resolve::{Group, Resolved, Skipped};
use std::collections::HashSet;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::process::Command;

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);

/// Combinaciones ya comprobadas con exito (Java + banderas): evita repetir el arranque en cada partida.
fn verified() -> &'static Mutex<HashSet<String>> {
    static CACHE: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cache_key(java: &Path, flags: &[String]) -> String {
    format!("{}|{}", java.display(), flags.join(" "))
}

/// `Ok(())` si la JVM arranca con esas banderas; `Err(motivo)` con la primera linea del error de Java si no.
pub async fn probe(java: &Path, flags: &[String]) -> Result<(), String> {
    if flags.is_empty() {
        return Ok(());
    }
    let key = cache_key(java, flags);
    if verified().lock().is_ok_and(|c| c.contains(&key)) {
        return Ok(());
    }
    let mut command = Command::new(java);
    command.args(flags).arg("-version").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).kill_on_drop(true);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW: sin consola parpadeante.
        command.creation_flags(0x0800_0000);
    }
    let run = tokio::time::timeout(PROBE_TIMEOUT, command.output()).await;
    let output = match run {
        Err(_) => return Err("Java tardo demasiado en responder".into()),
        Ok(Err(e)) => return Err(format!("no se pudo ejecutar Java: {e}")),
        Ok(Ok(output)) => output,
    };
    if output.status.success() {
        if let Ok(mut cache) = verified().lock() {
            cache.insert(key);
        }
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|l| l.contains("Unrecognized") || l.contains("Error") || l.contains("Invalid") || l.contains("Could not"))
        .or_else(|| stderr.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or("Java no acepto las opciones");
    Err(line.chars().take(160).collect())
}

/// Deja solo los bloques que la JVM acepta. Primero prueba todo junto (lo normal: una sola ejecucion); si falla,
/// va sumando bloques uno a uno y descarta los que rompen el arranque, para que un argumento mal escrito no impida jugar.
pub async fn keep_working(java: &Path, mut resolved: Resolved) -> Resolved {
    if probe(java, &resolved.flags()).await.is_ok() {
        return resolved;
    }
    let groups = std::mem::take(&mut resolved.groups);
    let mut accepted: Vec<Group> = Vec::new();
    for group in groups {
        let mut trial: Vec<String> = accepted.iter().flat_map(|g| g.flags.clone()).collect();
        trial.extend(group.flags.iter().cloned());
        match probe(java, &trial).await {
            Ok(()) => accepted.push(group),
            Err(reason) => {
                tracing::warn!(opcion = %group.id, %reason, "la JVM rechazo una opcion de rendimiento; se omite");
                resolved.skipped.push(Skipped { id: group.id.clone(), reason: format!("Java no la acepta: {reason}") });
            }
        }
    }
    resolved.groups = accepted;
    resolved
}

/// Valor que tiene una opcion de la JVM tras aplicar las banderas (`-XX:+PrintFlagsFinal`): demuestra que la bandera
/// surtio efecto y no solo que se acepto. `None` si Java no lista la opcion.
pub async fn final_value(java: &Path, flags: &[String], option: &str) -> Option<String> {
    let mut command = Command::new(java);
    command.args(flags).args(["-XX:+PrintFlagsFinal", "-version"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_0000);
    }
    let output = tokio::time::timeout(PROBE_TIMEOUT, command.output()).await.ok()?.ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| {
        // Formato: `     bool UseG1GC                                 := true    {product} {ergonomic}`
        let mut parts = line.split_whitespace();
        let _kind = parts.next()?;
        let name = parts.next()?;
        if name != option {
            return None;
        }
        // `=` valor por defecto; `:=` valor cambiado por una bandera o por la ergonomia.
        matches!(parts.next()?, "=" | ":=").then(|| parts.next().map(str::to_string)).flatten()
    })
}
