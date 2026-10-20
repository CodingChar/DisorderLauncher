use crate::InstanceError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_NAME_LEN: usize = 64;
/// Minecraft no arranca de forma fiable con menos heap que esto.
pub const MIN_MAX_MEMORY_MB: u32 = 512;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    /// Referencia a la cuenta. Nunca tokens (ver spec 06 §3).
    pub profile_id: String,
    pub java: JavaSelection,
    pub memory: MemoryPolicy,
    /// Icono elegido para la instancia (id de la interfaz, p. ej. `diamond-sword`). `None`: el del tipo.
    #[serde(default)]
    pub icon: Option<String>,
    /// Ajustes de rendimiento para equipos modestos.
    #[serde(default)]
    pub optimization: Optimization,
    /// Ajustes de JVM del catalogo (`jvm-tuning`) activados a mano, ademas de los del nivel de optimizacion.
    #[serde(default)]
    pub jvm_tweaks: Vec<String>,
    /// Argumentos de Java propios, separados por espacios (validados al guardar).
    #[serde(default)]
    pub jvm_args: String,
    /// Derivado de la ubicacion en disco; no se guarda en `instance.json`.
    pub game_dir: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoaderKind {
    Vanilla,
    Fabric,
    Forge,
    NeoForge,
    Quilt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum JavaSelection {
    Auto,
    Path { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum MemoryPolicy {
    Auto,
    #[serde(rename_all = "camelCase")]
    Fixed { min_mb: u32, max_mb: u32 },
}

#[derive(Debug, Clone)]
pub struct NewInstance {
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub profile_id: String,
}

/// Cambios parciales; `None` deja el campo como esta.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceUpdate {
    pub name: Option<String>,
    pub minecraft_version: Option<String>,
    pub profile_id: Option<String>,
    pub java: Option<JavaSelection>,
    pub memory: Option<MemoryPolicy>,
    /// Cadena vacia: vuelve al icono por defecto.
    pub icon: Option<String>,
    pub optimization: Option<Optimization>,
    pub jvm_tweaks: Option<Vec<String>>,
    pub jvm_args: Option<String>,
}

/// Nivel de optimizacion de una instancia: ajusta la JVM y las opciones graficas iniciales del juego.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Optimization {
    /// Sin cambios (valores de Minecraft).
    #[default]
    Off,
    /// Recolector de basura mas suave y graficos algo reducidos.
    Balanced,
    /// Para equipos modestos: lo minimo en graficos y la JVM mas ligera.
    LowEnd,
    /// Para equipos potentes (6 GB de RAM o mas): ajustes de memoria pensados para modpacks grandes.
    Performance,
}

pub(crate) fn validate_name(name: &str) -> Result<String, InstanceError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(InstanceError::Invalid("el nombre no puede estar vacio".into()));
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(InstanceError::Invalid(format!(
            "el nombre no puede superar {MAX_NAME_LEN} caracteres"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(InstanceError::Invalid("el nombre contiene caracteres no permitidos".into()));
    }
    Ok(name.to_string())
}

/// El icono es un id corto de la interfaz (`a-z`, `0-9`, `-`); nunca una ruta.
pub(crate) fn validate_icon(icon: &str) -> Result<Option<String>, InstanceError> {
    let icon = icon.trim();
    if icon.is_empty() {
        return Ok(None);
    }
    let ok = icon.len() <= 32 && icon.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !ok {
        return Err(InstanceError::Invalid("el icono no es valido".into()));
    }
    Ok(Some(icon.to_string()))
}

/// Ajustes de JVM: solo ids del catalogo, sin repetir.
pub(crate) fn validate_jvm_tweaks(tweaks: &[String]) -> Result<Vec<String>, InstanceError> {
    let mut out: Vec<String> = Vec::new();
    for id in tweaks {
        if jvm_tuning::find(id).is_none() {
            return Err(InstanceError::Invalid(format!("el ajuste de Java «{id}» no existe")));
        }
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    Ok(out)
}

/// Argumentos de Java propios: se normalizan a un solo espacio entre ellos y se rechaza lo peligroso.
pub(crate) fn validate_jvm_args(args: &str) -> Result<String, InstanceError> {
    let parsed = jvm_tuning::parse_args(args).map_err(|e| InstanceError::Invalid(format!("argumento de Java {e}")))?;
    Ok(parsed.join(" "))
}

pub(crate) fn validate_memory(memory: &MemoryPolicy) -> Result<(), InstanceError> {
    if let MemoryPolicy::Fixed { min_mb, max_mb } = memory {
        if *min_mb == 0 || min_mb > max_mb {
            return Err(InstanceError::Invalid(
                "la memoria minima debe ser mayor que 0 y no superar la maxima".into(),
            ));
        }
        if *max_mb < MIN_MAX_MEMORY_MB {
            return Err(InstanceError::Invalid(format!(
                "la memoria maxima debe ser de al menos {MIN_MAX_MEMORY_MB} MB"
            )));
        }
    }
    Ok(())
}

/// La version de Minecraft se usa como nombre de carpeta (`versions/<id>/`): solo letras, numeros,
/// espacio y `. - _ +`; sin `..`, separadores ni punto inicial. Admite ids como `1.21.4`, `24w14a`,
/// `b1.7.3` o `3D Shareware v1.34`.
pub(crate) fn validate_version(version: &str) -> Result<String, InstanceError> {
    let v = version.trim();
    let ok = !v.is_empty()
        && v.len() <= 64
        && !v.starts_with('.')
        && !v.contains("..")
        && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | ' '));
    if ok { Ok(v.to_string()) } else { Err(InstanceError::Invalid("la version de Minecraft no es valida".into())) }
}

pub(crate) fn validate_required(field: &str, value: &str) -> Result<(), InstanceError> {
    if value.trim().is_empty() {
        return Err(InstanceError::Invalid(format!("{field} es obligatorio")));
    }
    Ok(())
}
