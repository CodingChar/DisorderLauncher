//! Optimizacion de instancias para equipos modestos.
//!
//! Tres palancas, todas opcionales y reversibles:
//! 1. **JVM**: ajustes del catalogo `jvm-tuning` (recolector de basura, limpieza en paralelo...) que combinan el
//!    nivel de la instancia, los ajustes que la persona active y sus argumentos propios. Cada bandera se comprueba
//!    contra el Java real antes de lanzar (`jvm_tuning::keep_working`), asi que una opcion que ese Java no acepte se
//!    omite en vez de impedir el arranque. Las fuentes de cada ajuste estan en `docs/OPTIMIZATION.md`.
//! 2. **Opciones graficas iniciales** de Minecraft (`options.txt`): distancia de vision, particulas,
//!    sombras... Se aplican una sola vez por nivel; luego la persona puede cambiarlas en el juego.
//! 3. **Mods de rendimiento** de Modrinth (Sodium, Lithium, FerriteCore, EntityCulling, ModernFix...):
//!    se instalan a peticion en instancias con mods (`optimization_install_mods`).
//!
//! Fuentes de las recomendaciones (proyectos libres): Sodium/Lithium (CaffeineMC), FerriteCore,
//! ModernFix, EntityCulling, ImmediatelyFast, Embeddium (puerto de Sodium para Forge), Dynamic FPS y
//! MemoryLeakFix, todos en Modrinth.

use crate::{auto_memory_for_mods, count_mods, system_memory_mb, GameOutputEvent, Launcher, LauncherError, OutputStream};
use instance_manager::{Instance, LoaderKind, MemoryPolicy, Optimization};
use java_runtime::JavaInstallation;
use jvm_tuning::Env;
use serde::Serialize;
use std::path::Path;

const MARKER_FILE: &str = ".disorder-optimization";

/// Ajustes del catalogo que activa cada nivel. El nivel «Modesto» depende del equipo: con 2 hilos o menos, o menos
/// de 4 GB de RAM, la JVM ya elegiria el recolector simple (ergonomia de HotSpot) y aqui se pide de forma explicita;
/// con mas recursos G1 sigue siendo mejor opcion que un solo hilo de limpieza.
pub(crate) fn profile_tweaks(level: Optimization, env: &Env) -> Vec<&'static str> {
    match level {
        Optimization::Off => vec![],
        Optimization::Balanced => vec!["g1", "parallel-ref", "no-explicit-gc", "no-perfdata"],
        Optimization::Performance => vec!["g1", "parallel-ref", "no-explicit-gc", "no-perfdata", "g1-young", "code-cache"],
        Optimization::LowEnd if env.cpus <= 2 || env.ram_mb < 4096 => vec!["serial-gc", "no-explicit-gc", "no-perfdata"],
        Optimization::LowEnd => vec!["g1", "parallel-ref", "no-explicit-gc", "no-perfdata"],
    }
}

/// Lo que se sabe del equipo para decidir las banderas de una instancia.
pub(crate) fn jvm_env(instance: &Instance, java_major: u32) -> Env {
    let ram_mb = system_memory_mb();
    let heap_mb = match &instance.memory {
        MemoryPolicy::Fixed { max_mb, .. } => u64::from(*max_mb),
        MemoryPolicy::Auto => auto_memory_for_mods(ram_mb, count_mods(&instance.game_dir)).max_mb.map(u64::from).unwrap_or(1024),
    };
    let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
    Env { java_major, cpus, ram_mb, heap_mb }
}

/// Banderas finales sin comprobar contra Java: nivel + ajustes elegidos + argumentos propios.
pub(crate) fn plan_jvm(instance: &Instance, env: &Env) -> Result<jvm_tuning::Resolved, String> {
    let custom = jvm_tuning::parse_args(&instance.jvm_args).map_err(|e| e.to_string())?;
    let mut ids = profile_tweaks(instance.optimization, env);
    ids.extend(instance.jvm_tweaks.iter().map(String::as_str));
    Ok(jvm_tuning::resolve(&ids, &custom, env))
}

/// Opciones iniciales de `options.txt`. Las claves que una version no conoce las ignora el juego.
fn option_tweaks(level: Optimization) -> Vec<(&'static str, &'static str)> {
    match level {
        // Alto rendimiento no toca los graficos: solo la JVM.
        Optimization::Off | Optimization::Performance => vec![],
        Optimization::Balanced => vec![
            ("renderDistance", "8"),
            ("particles", "1"),
            ("maxFps", "120"),
            ("entityShadows", "false"),
            ("mipmapLevels", "2"),
            ("biomeBlendRadius", "1"),
        ],
        Optimization::LowEnd => vec![
            ("renderDistance", "5"),
            ("particles", "2"),
            ("maxFps", "60"),
            ("entityShadows", "false"),
            ("mipmapLevels", "0"),
            ("biomeBlendRadius", "0"),
            ("fancyGraphics", "false"),
            ("graphicsMode", "0"),
            ("ao", "false"),
            ("renderClouds", "\"false\""),
            ("clouds", "0"),
            ("enableVsync", "false"),
        ],
    }
}

/// `options.txt` con las claves cambiadas (las existentes se reemplazan, las que faltan se agregan).
fn apply_tweaks(current: &str, tweaks: &[(&str, &str)]) -> String {
    let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
    for (key, value) in tweaks {
        let prefix = format!("{key}:");
        match lines.iter().position(|l| l.starts_with(&prefix)) {
            Some(i) => lines[i] = format!("{key}:{value}"),
            None => lines.push(format!("{key}:{value}")),
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn level_name(level: Optimization) -> &'static str {
    match level {
        Optimization::Off => "off",
        Optimization::Performance => "performance",
        Optimization::Balanced => "balanced",
        Optimization::LowEnd => "lowEnd",
    }
}

/// Aplica las opciones graficas iniciales si este nivel aun no se aplico en la instancia.
pub(crate) async fn apply_initial_options(game_dir: &Path, level: Optimization) -> std::io::Result<()> {
    if option_tweaks(level).is_empty() {
        return Ok(());
    }
    let marker = game_dir.join(MARKER_FILE);
    if tokio::fs::read_to_string(&marker).await.is_ok_and(|m| m.trim() == level_name(level)) {
        return Ok(());
    }
    let options = game_dir.join("options.txt");
    let current = tokio::fs::read_to_string(&options).await.unwrap_or_default();
    tokio::fs::write(&options, apply_tweaks(&current, &option_tweaks(level))).await?;
    tokio::fs::write(&marker, level_name(level)).await
}

/// Mods de rendimiento recomendados para un tipo de instancia: `(slug en Modrinth, nombre)`.
pub(crate) fn recommended_mods(loader: LoaderKind, level: Optimization) -> Vec<(&'static str, &'static str)> {
    let mut mods: Vec<(&'static str, &'static str)> = match loader {
        LoaderKind::Fabric | LoaderKind::Quilt => vec![
            ("sodium", "Sodium (grafico)"),
            ("lithium", "Lithium (logica del juego)"),
            ("ferrite-core", "FerriteCore (memoria)"),
            ("entityculling", "Entity Culling"),
            ("modernfix", "ModernFix"),
            ("immediatelyfast", "ImmediatelyFast"),
        ],
        LoaderKind::Forge => vec![
            ("embeddium", "Embeddium (grafico)"),
            ("ferrite-core", "FerriteCore (memoria)"),
            ("modernfix", "ModernFix"),
            ("entityculling", "Entity Culling"),
        ],
        LoaderKind::NeoForge => vec![
            ("sodium", "Sodium (grafico)"),
            ("ferrite-core", "FerriteCore (memoria)"),
            ("modernfix", "ModernFix"),
            ("entityculling", "Entity Culling"),
            ("immediatelyfast", "ImmediatelyFast"),
        ],
        LoaderKind::Vanilla => vec![],
    };
    if level == Optimization::LowEnd && !mods.is_empty() {
        mods.push(("dynamic-fps", "Dynamic FPS (baja los FPS en segundo plano)"));
        mods.push(("memoryleakfix", "MemoryLeakFix"));
    }
    mods
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptimizationReport {
    /// Nombres de los mods instalados ahora.
    pub installed: Vec<String>,
    /// Los que no se pudieron instalar (no existen para esa version, etc.), con el motivo.
    pub skipped: Vec<SkippedMod>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedMod {
    pub name: String,
    pub reason: String,
}

/// Un ajuste del catalogo tal como lo muestra la pantalla de opciones.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmTweakInfo {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub risk: jvm_tuning::Risk,
    pub source: String,
    /// Activado a mano para esta instancia.
    pub selected: bool,
    /// Ya lo aplica el nivel de rendimiento elegido.
    pub from_level: bool,
    /// Motivo por el que no sirve en este Java o equipo (`None`: disponible).
    pub unavailable: Option<String>,
}

/// Resultado de comprobar los ajustes y argumentos de Java de una instancia.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JvmCheck {
    pub java_major: u32,
    /// `true` si se probaron contra el Java instalado; `false` si ese Java aun no esta descargado.
    pub verified: bool,
    pub tweaks: Vec<JvmTweakInfo>,
    /// Banderas que se pasaran a Java.
    pub flags: Vec<String>,
    pub skipped: Vec<jvm_tuning::Skipped>,
}

impl Launcher {
    /// JVM y opciones graficas segun el nivel de la instancia (se llama al preparar el lanzamiento).
    /// Devuelve las banderas de la JVM ya comprobadas contra el Java que se va a usar.
    pub(crate) async fn apply_optimization(&self, instance: &Instance, java: &JavaInstallation) -> Vec<String> {
        if let Err(e) = apply_initial_options(&instance.game_dir, instance.optimization).await {
            tracing::warn!(error = %e, "no se pudieron aplicar las opciones graficas de optimizacion");
        }
        let env = jvm_env(instance, java.major);
        let resolved = match plan_jvm(instance, &env) {
            Ok(resolved) => resolved,
            Err(reason) => {
                // `instance.json` editado a mano: se juega sin los argumentos propios.
                self.console_note(&instance.id, &format!("Se ignoraron los argumentos de Java de esta instancia: {reason}"));
                let ids = profile_tweaks(instance.optimization, &env);
                jvm_tuning::resolve(&ids, &[], &env)
            }
        };
        let verified = jvm_tuning::keep_working(&java.executable(), resolved).await;
        for skipped in &verified.skipped {
            // Los que no aplican por version o equipo son esperables; solo se avisa de lo que la JVM rechazo.
            if skipped.reason.starts_with("Java no la acepta") {
                self.console_note(&instance.id, &format!("Se omitio la opcion de Java «{}»: {}", skipped.id, skipped.reason));
            }
        }
        verified.flags()
    }

    /// Linea informativa del launcher en la consola de la instancia.
    fn console_note(&self, instance_id: &str, text: &str) {
        self.events.game_output(
            OutputStream::Stderr,
            GameOutputEvent { instance_id: instance_id.to_string(), line: format!("[DisorderLauncher] {text}") },
        );
    }

    /// Comprueba unos ajustes y argumentos (aun sin guardar) para una instancia: que se aplicarian y si el Java
    /// instalado los acepta. No descarga nada: sin el Java exacto instalado solo se devuelve la decision teorica.
    pub async fn jvm_options_check(
        &self,
        instance_id: &str,
        level: Optimization,
        tweaks: Vec<String>,
        args: String,
    ) -> Result<JvmCheck, LauncherError> {
        let mut instance = self
            .instances_list()
            .await?
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| LauncherError::InvalidInput("No se encontro la instancia.".into()))?;
        instance.optimization = level;
        instance.jvm_tweaks = tweaks;
        instance.jvm_args = args;
        jvm_tuning::parse_args(&instance.jvm_args).map_err(|e| LauncherError::InvalidInput(format!("Argumento de Java {e}")))?;
        for id in &instance.jvm_tweaks {
            if jvm_tuning::find(id).is_none() {
                return Err(LauncherError::InvalidInput(format!("El ajuste de Java «{id}» no existe.")));
            }
        }

        let required = match self.resolve_descriptor(&instance.minecraft_version).await {
            Ok(descriptor) => descriptor.java_version.as_ref().map(|j| j.major_version),
            Err(_) => None,
        };
        let installed = self.java.detect_installed().await;
        let java = match (&instance.java, required) {
            (instance_manager::JavaSelection::Path { path }, _) => java_runtime::detect_java_at(path).await.ok(),
            (_, Some(major)) => installed.iter().find(|j| j.major == major).cloned(),
            (_, None) => None,
        };
        let java_major = java.as_ref().map(|j| j.major).or(required).unwrap_or(17);
        let env = jvm_env(&instance, java_major);

        let resolved = plan_jvm(&instance, &env).map_err(LauncherError::InvalidInput)?;
        let (resolved, verified) = match &java {
            Some(java) => (jvm_tuning::keep_working(&java.executable(), resolved).await, true),
            None => (resolved, false),
        };
        let selected: Vec<&str> = instance.jvm_tweaks.iter().map(String::as_str).collect();
        let profile = profile_tweaks(instance.optimization, &env);
        let entries = jvm_tuning::CATALOG
            .iter()
            .map(|t| JvmTweakInfo {
                id: t.id.to_string(),
                label: t.label.to_string(),
                hint: t.hint.to_string(),
                risk: t.risk,
                source: t.source.to_string(),
                selected: selected.contains(&t.id),
                from_level: profile.contains(&t.id),
                unavailable: jvm_tuning::unavailable_reason(t, &env),
            })
            .collect();
        Ok(JvmCheck { java_major, verified, tweaks: entries, flags: resolved.flags(), skipped: resolved.skipped })
    }

    /// Instala los mods de rendimiento recomendados para la instancia (necesita internet).
    /// Cada mod se prueba por separado: los que no existen para esa version se informan y se omiten.
    pub async fn optimization_install_mods(&self, instance_id: &str) -> Result<OptimizationReport, LauncherError> {
        let instance = self
            .instances_list()
            .await?
            .into_iter()
            .find(|i| i.id == instance_id)
            .ok_or_else(|| LauncherError::InvalidInput("No se encontro la instancia.".into()))?;
        let list = recommended_mods(instance.loader, instance.optimization);
        if list.is_empty() {
            return Err(LauncherError::InvalidInput(
                "Esta instancia no admite mods. Crea una con Fabric, Quilt, Forge o NeoForge para añadir mods de rendimiento.".into(),
            ));
        }

        let mut report = OptimizationReport { installed: Vec::new(), skipped: Vec::new() };
        for (slug, name) in list {
            match self.mods_install(instance_id, slug).await {
                Ok(_) => report.installed.push(name.to_string()),
                // Sin internet no tiene sentido seguir probando uno por uno.
                Err(LauncherError::Offline) => return Err(LauncherError::Offline),
                Err(e) => report.skipped.push(SkippedMod { name: name.to_string(), reason: e.to_string() }),
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(cpus: usize, ram_mb: u64) -> Env {
        Env { java_major: 21, cpus, ram_mb, heap_mb: 4096 }
    }

    #[test]
    fn levels_pick_their_tweaks() {
        let e = env(8, 16384);
        assert!(profile_tweaks(Optimization::Off, &e).is_empty());
        assert!(profile_tweaks(Optimization::Balanced, &e).contains(&"g1"));
        let performance = profile_tweaks(Optimization::Performance, &e);
        assert!(performance.contains(&"g1-young") && performance.contains(&"code-cache"));
        assert!(!profile_tweaks(Optimization::Balanced, &e).contains(&"g1-young"));
    }

    #[test]
    fn low_end_uses_the_simple_collector_only_on_weak_machines() {
        assert!(profile_tweaks(Optimization::LowEnd, &env(2, 8192)).contains(&"serial-gc"));
        assert!(profile_tweaks(Optimization::LowEnd, &env(8, 2048)).contains(&"serial-gc"));
        let strong = profile_tweaks(Optimization::LowEnd, &env(4, 8192));
        assert!(strong.contains(&"g1") && !strong.contains(&"serial-gc"));
    }

    fn instance_with(level: Optimization, tweaks: &[&str], args: &str) -> Instance {
        Instance {
            id: "inst_x".into(),
            name: "x".into(),
            minecraft_version: "1.21.1".into(),
            loader: LoaderKind::Fabric,
            loader_version: None,
            profile_id: "p".into(),
            java: instance_manager::JavaSelection::Auto,
            memory: MemoryPolicy::Fixed { min_mb: 1024, max_mb: 4096 },
            icon: None,
            optimization: level,
            jvm_tweaks: tweaks.iter().map(|s| s.to_string()).collect(),
            jvm_args: args.into(),
            game_dir: std::path::PathBuf::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn plan_combines_level_extra_tweaks_and_custom_args() {
        let e = env(8, 16384);
        let inst = instance_with(Optimization::Balanced, &["string-dedup"], "-XX:MaxGCPauseMillis=120");
        let flags = plan_jvm(&inst, &e).unwrap().flags();
        for expected in ["-XX:+UseG1GC", "-XX:+DisableExplicitGC", "-XX:+UseStringDeduplication", "-XX:MaxGCPauseMillis=120"] {
            assert!(flags.contains(&expected.to_string()), "falta {expected}");
        }
        // Los argumentos propios van al final: en la JVM la ultima bandera manda.
        assert_eq!(flags.last().unwrap(), "-XX:MaxGCPauseMillis=120");
    }

    #[test]
    fn off_level_adds_nothing_unless_the_user_asks() {
        let e = env(8, 16384);
        assert!(plan_jvm(&instance_with(Optimization::Off, &[], ""), &e).unwrap().flags().is_empty());
        assert_eq!(plan_jvm(&instance_with(Optimization::Off, &["no-perfdata"], ""), &e).unwrap().flags(), ["-XX:-UsePerfData"]);
    }

    #[test]
    fn hand_edited_dangerous_args_are_refused_by_the_plan() {
        let e = env(8, 16384);
        assert!(plan_jvm(&instance_with(Optimization::Off, &[], "-javaagent:x.jar"), &e).is_err());
    }

    #[test]
    fn tweaks_replace_existing_keys_and_add_missing_ones() {
        let out = apply_tweaks("lang:es_es\nrenderDistance:16\nfov:0.5\n", &[("renderDistance", "5"), ("particles", "2")]);
        assert_eq!(out, "lang:es_es\nrenderDistance:5\nfov:0.5\nparticles:2\n");
    }

    #[tokio::test]
    async fn initial_options_are_applied_once_per_level() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("options.txt"), "renderDistance:16\n").unwrap();

        apply_initial_options(dir.path(), Optimization::LowEnd).await.unwrap();
        let first = std::fs::read_to_string(dir.path().join("options.txt")).unwrap();
        assert!(first.contains("renderDistance:5") && first.contains("particles:2"));

        // La persona sube la distancia en el juego: no se vuelve a pisar con el mismo nivel.
        std::fs::write(dir.path().join("options.txt"), "renderDistance:12\n").unwrap();
        apply_initial_options(dir.path(), Optimization::LowEnd).await.unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("options.txt")).unwrap(), "renderDistance:12\n");

        // Cambiar de nivel si vuelve a aplicar.
        apply_initial_options(dir.path(), Optimization::Balanced).await.unwrap();
        assert!(std::fs::read_to_string(dir.path().join("options.txt")).unwrap().contains("renderDistance:8"));

        // Apagado: no toca nada.
        std::fs::write(dir.path().join("options.txt"), "renderDistance:20\n").unwrap();
        apply_initial_options(dir.path(), Optimization::Off).await.unwrap();
        assert_eq!(std::fs::read_to_string(dir.path().join("options.txt")).unwrap(), "renderDistance:20\n");
    }

    #[test]
    fn recommended_mods_follow_the_loader() {
        assert!(recommended_mods(LoaderKind::Vanilla, Optimization::LowEnd).is_empty());
        assert!(recommended_mods(LoaderKind::Forge, Optimization::Balanced).iter().any(|(s, _)| *s == "embeddium"));
        assert!(recommended_mods(LoaderKind::Fabric, Optimization::Balanced).iter().any(|(s, _)| *s == "sodium"));
        assert!(recommended_mods(LoaderKind::Fabric, Optimization::LowEnd).iter().any(|(s, _)| *s == "dynamic-fps"));
        assert!(!recommended_mods(LoaderKind::Fabric, Optimization::Balanced).iter().any(|(s, _)| *s == "dynamic-fps"));
    }
}
