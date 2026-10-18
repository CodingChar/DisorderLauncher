use crate::{LaunchError, Placeholders};
use identity_api::ResolvedIdentity;
use java_runtime::JavaInstallation;
use minecraft_meta::rules::{rules_allow_with, RuleFeatures};
use minecraft_meta::{
    library_key, resolve_library, ArgumentValue, ArgumentValueInner, Arguments, Library, VersionDescriptor,
};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

/// Secretos de menos de 8 caracteres (p. ej. el token `0` de Offline Gaming) no
/// se redactan en la salida del juego: reemplazarian texto legitimo.
const MIN_REDACTABLE_SECRET_LEN: usize = 8;

#[derive(Clone)]
pub struct LaunchPlan {
    pub java_path: PathBuf,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
    /// Valores que no deben aparecer en logs ni en la salida capturada.
    pub(crate) secrets: Vec<String>,
}

impl LaunchPlan {
    pub fn secrets(&self) -> &[String] {
        &self.secrets
    }

    /// Argv completo (sin el ejecutable): JVM, classpath, clase principal y juego.
    pub fn command_args(&self) -> Result<Vec<String>, LaunchError> {
        let classpath = std::env::join_paths(&self.classpath)
            .map_err(|e| LaunchError::InvalidClasspath(e.to_string()))?
            .to_string_lossy()
            .into_owned();

        let mut args = Vec::with_capacity(self.jvm_args.len() + self.game_args.len() + 3);
        args.extend(self.jvm_args.iter().cloned());
        args.push("-cp".to_string());
        args.push(classpath);
        args.push(self.main_class.clone());
        args.extend(self.game_args.iter().cloned());
        Ok(args)
    }

    /// Comprueba que Java y todos los archivos del classpath existen antes de lanzar,
    /// para poder explicar al usuario que falta en lugar de fallar dentro de la JVM.
    pub fn validate(&self) -> Result<(), LaunchError> {
        if !self.java_path.is_file() {
            return Err(LaunchError::JavaMissing(self.java_path.clone()));
        }
        let missing: Vec<PathBuf> = self
            .classpath
            .iter()
            .filter(|p| !p.is_file())
            .cloned()
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(LaunchError::MissingFiles(missing))
        }
    }
}

// El plan contiene el token de sesion dentro de `game_args`; nunca se imprime.
impl fmt::Debug for LaunchPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaunchPlan")
            .field("java_path", &self.java_path)
            .field("main_class", &self.main_class)
            .field("classpath_entries", &self.classpath.len())
            .field("jvm_args", &self.jvm_args.len())
            .field("game_args", &"<redacted>")
            .field("working_directory", &self.working_directory)
            .finish()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemorySettings {
    pub min_mb: Option<u32>,
    pub max_mb: Option<u32>,
}

/// Cambios que un loader (Fabric, Quilt...) aplica sobre una version vanilla: otra clase
/// principal, librerias propias (que tienen prioridad en el classpath) y argumentos extra.
#[derive(Debug, Clone, Default)]
pub struct LaunchOverlay {
    pub main_class: Option<String>,
    pub libraries: Vec<Library>,
    pub arguments: Option<Arguments>,
    /// Argumentos de juego como cadena (formato antiguo); si existe, sustituye a los de vanilla.
    pub minecraft_arguments: Option<String>,
}

/// Datos de entorno del launcher que no vienen de la version ni de la identidad.
#[derive(Debug, Clone)]
pub struct LaunchContext {
    /// Raiz compartida con `versions/`, `libraries/` y `assets/`.
    pub data_dir: PathBuf,
    /// Directorio de juego de la instancia (saves, mods, options).
    pub game_dir: PathBuf,
    pub memory: MemorySettings,
    /// Banderas de la JVM que agrega el launcher (p. ej. optimizacion para equipos modestos).
    pub extra_jvm_args: Vec<String>,
    pub window_size: Option<(u32, u32)>,
    /// Valor de `${game_assets}` cuando el indice de assets es legacy
    /// (`map_to_resources` -> `<gameDir>/resources`). Por defecto `assets/virtual/<indice>`.
    pub game_assets_dir: Option<PathBuf>,
    pub overlay: Option<LaunchOverlay>,
    pub launcher_name: String,
    pub launcher_version: String,
}

impl LaunchContext {
    pub fn new(data_dir: impl Into<PathBuf>, game_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            game_dir: game_dir.into(),
            memory: MemorySettings::default(),
            extra_jvm_args: Vec::new(),
            window_size: None,
            game_assets_dir: None,
            overlay: None,
            launcher_name: "DisorderLauncher".to_string(),
            launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub struct LaunchPlanBuilder;

impl LaunchPlanBuilder {
    /// Construye el plan de lanzamiento. Es independiente del proveedor de
    /// identidad: solo consume un `ResolvedIdentity`.
    pub fn build(
        version: &VersionDescriptor,
        identity: &ResolvedIdentity,
        java: &JavaInstallation,
        ctx: &LaunchContext,
    ) -> Result<LaunchPlan, LaunchError> {
        let versions_dir = ctx.data_dir.join("versions").join(&version.id);
        let natives_dir = versions_dir.join("natives");
        let assets_root = ctx.data_dir.join("assets");
        let client_jar = versions_dir.join(format!("{}.jar", version.id));

        let classpath = build_classpath(version, ctx.overlay.as_ref(), &ctx.data_dir, client_jar);

        let mut features = RuleFeatures::none();
        if ctx.window_size.is_some() {
            features = features.with("has_custom_resolution");
        }

        let mut placeholders = Placeholders::new();
        placeholders.set("auth_player_name", &identity.display_name);
        placeholders.set("auth_uuid", &identity.game_uuid);
        placeholders.set("auth_access_token", &identity.session.access_token);
        placeholders.set("auth_session", &identity.session.access_token);
        placeholders.set("user_type", &identity.session.token_type);
        placeholders.set("user_properties", "{}");
        placeholders.set("clientid", "0");
        placeholders.set("auth_xuid", "0");
        placeholders.set("version_name", &version.id);
        placeholders.set("version_type", &version.version_type);
        placeholders.set("game_directory", path_str(&ctx.game_dir));
        placeholders.set("assets_root", path_str(&assets_root));
        placeholders.set("assets_index_name", &version.asset_index.id);
        let game_assets = ctx
            .game_assets_dir
            .clone()
            .unwrap_or_else(|| assets_root.join("virtual").join(&version.asset_index.id));
        placeholders.set("game_assets", path_str(&game_assets));
        placeholders.set("natives_directory", path_str(&natives_dir));
        placeholders.set("library_directory", path_str(&ctx.data_dir.join("libraries")));
        placeholders.set("classpath_separator", if cfg!(windows) { ";" } else { ":" });
        placeholders.set("launcher_name", &ctx.launcher_name);
        placeholders.set("launcher_version", &ctx.launcher_version);
        if let Some((w, h)) = ctx.window_size {
            placeholders.set("resolution_width", w.to_string());
            placeholders.set("resolution_height", h.to_string());
        }

        let (mut raw_jvm, mut raw_game) = collect_raw_args(version, &features)?;
        if let Some(legacy) = ctx.overlay.as_ref().and_then(|o| o.minecraft_arguments.as_deref()) {
            raw_game = legacy.split_whitespace().map(str::to_string).collect();
        }
        if let Some(extra) = ctx.overlay.as_ref().and_then(|o| o.arguments.as_ref()) {
            raw_jvm.extend(expand_arguments(&extra.jvm, &features));
            raw_game.extend(expand_arguments(&extra.game, &features));
        }

        let mut jvm_args = Vec::new();
        if let Some(min) = ctx.memory.min_mb {
            jvm_args.push(format!("-Xms{min}M"));
        }
        if let Some(max) = ctx.memory.max_mb {
            jvm_args.push(format!("-Xmx{max}M"));
        }
        jvm_args.extend(ctx.extra_jvm_args.iter().cloned());
        // `-cp`/`${classpath}` los emite `command_args` a partir de `classpath`.
        for raw in strip_classpath_args(raw_jvm) {
            jvm_args.push(placeholders.apply(&raw)?);
        }

        let game_args = raw_game
            .iter()
            .map(|raw| placeholders.apply(raw))
            .collect::<Result<Vec<_>, _>>()?;

        let secrets = [&identity.session.access_token]
            .into_iter()
            .filter(|s| s.len() >= MIN_REDACTABLE_SECRET_LEN)
            .cloned()
            .collect();

        Ok(LaunchPlan {
            java_path: java.executable(),
            main_class: ctx
                .overlay
                .as_ref()
                .and_then(|o| o.main_class.clone())
                .unwrap_or_else(|| version.main_class.clone()),
            classpath,
            jvm_args,
            game_args,
            working_directory: ctx.game_dir.clone(),
            environment: HashMap::new(),
            secrets,
        })
    }
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn build_classpath(
    version: &VersionDescriptor,
    overlay: Option<&LaunchOverlay>,
    data_dir: &Path,
    client_jar: PathBuf,
) -> Vec<PathBuf> {
    let libraries_dir = data_dir.join("libraries");
    let mut classpath: Vec<PathBuf> = Vec::new();

    let overlay_libs = overlay.map(|o| o.libraries.as_slice()).unwrap_or_default();
    let overlay_keys: HashSet<String> = overlay_libs.iter().map(|l| library_key(&l.name)).collect();

    let mut push = |lib: &Library| {
        if let Some(rules) = &lib.rules {
            if !rules_allow_with(rules, &RuleFeatures::none()) {
                return;
            }
        }
        let Some(resolved) = resolve_library(lib) else { return };
        let path = libraries_dir.join(&resolved.path);
        if !classpath.contains(&path) {
            classpath.push(path);
        }
    };

    for lib in overlay_libs {
        push(lib);
    }
    // La version del loader gana: se omite la libreria vanilla equivalente.
    for lib in version.libraries.iter().filter(|l| !overlay_keys.contains(&library_key(&l.name))) {
        push(lib);
    }

    classpath.push(client_jar);
    classpath
}

/// Devuelve (jvm, game) sin sustituir. Soporta el formato moderno (`arguments`) y el
/// legado (`minecraftArguments`, con JVM por defecto).
fn collect_raw_args(
    version: &VersionDescriptor,
    features: &RuleFeatures,
) -> Result<(Vec<String>, Vec<String>), LaunchError> {
    if let Some(arguments) = &version.arguments {
        return Ok((
            expand_arguments(&arguments.jvm, features),
            expand_arguments(&arguments.game, features),
        ));
    }

    let legacy = version
        .minecraft_arguments
        .as_deref()
        .ok_or(LaunchError::MissingGameArguments)?;

    let jvm = vec![
        "-Djava.library.path=${natives_directory}".to_string(),
        "-Dminecraft.launcher.brand=${launcher_name}".to_string(),
        "-Dminecraft.launcher.version=${launcher_version}".to_string(),
    ];
    let game = legacy.split_whitespace().map(str::to_string).collect();
    Ok((jvm, game))
}

fn expand_arguments(values: &[ArgumentValue], features: &RuleFeatures) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        match value {
            ArgumentValue::Simple(s) => out.push(s.clone()),
            ArgumentValue::Conditional { rules, value } => {
                if !rules_allow_with(rules, features) {
                    continue;
                }
                match value {
                    ArgumentValueInner::Single(s) => out.push(s.clone()),
                    ArgumentValueInner::Multiple(list) => out.extend(list.iter().cloned()),
                }
            }
        }
    }
    out
}

fn strip_classpath_args(args: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        if arg == "-cp" || arg == "-classpath" || arg == "--class-path" {
            iter.next();
            continue;
        }
        out.push(arg);
    }
    out
}
