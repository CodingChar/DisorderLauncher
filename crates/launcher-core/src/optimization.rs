//! Optimizacion de instancias para equipos modestos.
//!
//! Tres palancas, todas opcionales y reversibles:
//! 1. **JVM**: banderas del recolector de basura y menos hilos de apoyo (`jvm_flags`).
//! 2. **Opciones graficas iniciales** de Minecraft (`options.txt`): distancia de vision, particulas,
//!    sombras... Se aplican una sola vez por nivel; luego la persona puede cambiarlas en el juego.
//! 3. **Mods de rendimiento** de Modrinth (Sodium, Lithium, FerriteCore, EntityCulling, ModernFix...):
//!    se instalan a peticion en instancias con mods (`optimization_install_mods`).
//!
//! Fuentes de las recomendaciones (proyectos libres): Sodium/Lithium (CaffeineMC), FerriteCore,
//! ModernFix, EntityCulling, ImmediatelyFast, Embeddium (puerto de Sodium para Forge), Dynamic FPS y
//! MemoryLeakFix, todos en Modrinth.

use crate::{Launcher, LauncherError};
use instance_manager::{Instance, LoaderKind, Optimization};
use serde::Serialize;
use std::path::Path;

const MARKER_FILE: &str = ".disorder-optimization";

/// Banderas de la JVM validas desde Java 8 hasta las versiones actuales.
pub(crate) fn jvm_flags(level: Optimization) -> Vec<String> {
    let flags: &[&str] = match level {
        Optimization::Off => &[],
        // G1 con pausas cortas: menos tirones sin gastar mas memoria.
        Optimization::Balanced => &[
            "-XX:+UseG1GC",
            "-XX:+ParallelRefProcEnabled",
            "-XX:MaxGCPauseMillis=100",
            "-XX:+DisableExplicitGC",
            "-XX:-UsePerfData",
        ],
        // Recolector serie: un solo hilo de GC, ideal en procesadores de 2 nucleos o poca RAM.
        Optimization::LowEnd => &["-XX:+UseSerialGC", "-XX:+DisableExplicitGC", "-XX:-UsePerfData", "-Xshare:auto"],
    };
    flags.iter().map(|s| s.to_string()).collect()
}

/// Opciones iniciales de `options.txt`. Las claves que una version no conoce las ignora el juego.
fn option_tweaks(level: Optimization) -> Vec<(&'static str, &'static str)> {
    match level {
        Optimization::Off => vec![],
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
        Optimization::Balanced => "balanced",
        Optimization::LowEnd => "lowEnd",
    }
}

/// Aplica las opciones graficas iniciales si este nivel aun no se aplico en la instancia.
pub(crate) async fn apply_initial_options(game_dir: &Path, level: Optimization) -> std::io::Result<()> {
    if level == Optimization::Off {
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

impl Launcher {
    /// JVM y opciones graficas segun el nivel de la instancia (se llama al preparar el lanzamiento).
    pub(crate) async fn apply_optimization(&self, instance: &Instance) -> Vec<String> {
        if let Err(e) = apply_initial_options(&instance.game_dir, instance.optimization).await {
            tracing::warn!(error = %e, "no se pudieron aplicar las opciones graficas de optimizacion");
        }
        jvm_flags(instance.optimization)
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

    #[test]
    fn jvm_flags_depend_on_the_level() {
        assert!(jvm_flags(Optimization::Off).is_empty());
        assert!(jvm_flags(Optimization::Balanced).contains(&"-XX:+UseG1GC".to_string()));
        let low = jvm_flags(Optimization::LowEnd);
        assert!(low.contains(&"-XX:+UseSerialGC".to_string()));
        assert!(!low.contains(&"-XX:+UseG1GC".to_string()));
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
