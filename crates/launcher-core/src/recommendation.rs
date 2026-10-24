//! Recomendacion de rendimiento para una instancia segun el equipo y los mods que carga.
//!
//! No cambia nada por si sola: devuelve el nivel de optimizacion y la memoria que mejor encajan, con los
//! motivos en palabras simples, para que la persona los aplique con un toque.

use crate::{auto_memory_for_mods, count_mods, system_memory_mb, Launcher, LauncherError};
use instance_manager::{InstanceRepository, Optimization};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub optimization: Optimization,
    /// Memoria maxima sugerida en MB.
    pub memory_mb: u32,
    /// Por que se sugiere eso, una frase por motivo.
    pub reasons: Vec<String>,
    pub ram_mb: u64,
    pub cpus: usize,
    pub mods: usize,
}

/// Decide el nivel segun hilos de CPU, RAM y cantidad de mods. Pura: facil de probar.
pub(crate) fn recommend(ram_mb: u64, cpus: usize, mods: usize) -> Recommendation {
    let mut reasons = Vec::new();
    let optimization = if ram_mb < 4096 || cpus <= 2 {
        reasons.push("Tu equipo tiene poca memoria o pocos nucleos: conviene aligerar el juego al maximo.".to_string());
        Optimization::LowEnd
    } else if ram_mb < 8192 || cpus <= 4 {
        reasons.push("Tu equipo es de gama media: un ajuste equilibrado da mas fluidez sin perder calidad.".to_string());
        Optimization::Balanced
    } else if mods >= 100 {
        reasons.push("Tu equipo es potente y la instancia carga muchos mods: se afina la memoria para modpacks grandes.".to_string());
        Optimization::Performance
    } else {
        reasons.push("Tu equipo es potente y la instancia es ligera: no hace falta tocar nada.".to_string());
        Optimization::Off
    };

    let memory = auto_memory_for_mods(ram_mb, mods);
    let memory_mb = memory.max_mb.unwrap_or(2048);
    if mods >= 20 {
        reasons.push(format!("Con {mods} mods se reserva mas memoria ({} GB como maximo).", memory_mb.div_ceil(1024)));
    }
    if ram_mb > 0 && u64::from(memory_mb) * 10 >= ram_mb * 6 {
        reasons.push("La memoria se limita al 60 % de la de tu equipo para que Windows siga fluido.".to_string());
    }
    Recommendation { optimization, memory_mb, reasons, ram_mb, cpus, mods }
}

impl Launcher {
    pub async fn instances_recommendation(&self, instance_id: &str) -> Result<Recommendation, LauncherError> {
        let instance = self.instances.get(instance_id).await?.ok_or(LauncherError::InstanceNotFound)?;
        let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2);
        Ok(recommend(system_memory_mb(), cpus, count_mods(&instance.game_dir)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_follows_the_machine_and_the_pack() {
        assert_eq!(recommend(3072, 4, 0).optimization, Optimization::LowEnd);
        assert_eq!(recommend(16384, 2, 0).optimization, Optimization::LowEnd);
        // i3 de 4 hilos con ~10 GB: gama media.
        assert_eq!(recommend(10240, 4, 135).optimization, Optimization::Balanced);
        assert_eq!(recommend(6144, 8, 10).optimization, Optimization::Balanced);
        assert_eq!(recommend(16384, 8, 135).optimization, Optimization::Performance);
        assert_eq!(recommend(16384, 8, 5).optimization, Optimization::Off);
    }

    #[test]
    fn memory_grows_with_mods_and_always_explains_itself() {
        let big = recommend(10240, 4, 135);
        assert_eq!(big.memory_mb, 4748);
        assert!(big.reasons.iter().any(|r| r.contains("135 mods")));
        assert!(recommend(16384, 8, 0).memory_mb < big.memory_mb || recommend(16384, 8, 0).memory_mb == 4096);
        assert!(!recommend(8192, 4, 0).reasons.is_empty());
    }
}
