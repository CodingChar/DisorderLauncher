//! Convierte «ajustes pedidos + argumentos propios» en la lista final de banderas para un Java y un equipo concretos,
//! explicando lo que se descarta y por que.

use crate::args::{key, selected_gc};
use crate::catalog::{find, Env, Gc, Tweak};
use serde::Serialize;

/// Un ajuste (o argumento) que no se aplica, con el motivo en lenguaje llano.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

/// Bloque de banderas que se prueban y se descartan juntos (un ajuste, o un argumento propio).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub flags: Vec<String>,
    /// `true` si viene de los argumentos escritos por la persona.
    pub custom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Resolved {
    pub groups: Vec<Group>,
    pub skipped: Vec<Skipped>,
}

impl Resolved {
    /// Banderas finales en orden, sin repetir ninguna igual.
    pub fn flags(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for flag in self.groups.iter().flat_map(|g| &g.flags) {
            if !out.contains(flag) {
                out.push(flag.clone());
            }
        }
        out
    }

    pub fn applied_ids(&self) -> Vec<&str> {
        self.groups.iter().filter(|g| !g.custom).map(|g| g.id.as_str()).collect()
    }
}

/// Por que un ajuste no sirve en este equipo, o `None` si es aplicable (sin mirar recolectores).
pub fn unavailable_reason(tweak: &Tweak, env: &Env) -> Option<String> {
    if env.java_major < tweak.min_java {
        return Some(format!("necesita Java {} o superior (se usa Java {})", tweak.min_java, env.java_major));
    }
    if tweak.max_java.is_some_and(|max| env.java_major > max) {
        return Some(format!("no es compatible con Java {}", env.java_major));
    }
    if env.ram_mb < tweak.min_ram_mb {
        return Some(format!("necesita un equipo con {} GB de RAM o mas", tweak.min_ram_mb / 1024));
    }
    None
}

/// `requested`: ids del catalogo (perfil + elegidos); `custom`: argumentos ya validados.
pub fn resolve(requested: &[&str], custom: &[String], env: &Env) -> Resolved {
    let mut out = Resolved::default();

    // Recolector que fijan los argumentos propios: manda sobre los ajustes.
    let mut custom_gc: Option<Gc> = None;
    let mut custom_has_unknown_gc = false;
    for flag in custom {
        match selected_gc(flag) {
            Ok(Some(gc)) => custom_gc = Some(gc),
            Ok(None) => {}
            Err(_) => custom_has_unknown_gc = true,
        }
    }

    // Primer paso: ajustes validos por version/equipo, sin repetidos.
    let mut candidates: Vec<&Tweak> = Vec::new();
    for id in requested {
        if candidates.iter().any(|t| t.id == *id) {
            continue;
        }
        match find(id) {
            None => out.skipped.push(Skipped { id: (*id).into(), reason: "ajuste desconocido".into() }),
            Some(tweak) => match unavailable_reason(tweak, env) {
                Some(reason) => out.skipped.push(Skipped { id: tweak.id.into(), reason }),
                None => candidates.push(tweak),
            },
        }
    }

    // Segundo: un solo recolector. Los propios argumentos ganan; entre ajustes, el primero.
    let mut chosen_gc: Option<Gc> = None;
    let mut kept: Vec<&Tweak> = Vec::new();
    for tweak in candidates {
        if let Some(gc) = tweak.selects_gc {
            if custom_gc.is_some() || custom_has_unknown_gc {
                out.skipped.push(Skipped { id: tweak.id.into(), reason: "tus argumentos ya eligen otro recolector de memoria".into() });
                continue;
            }
            if chosen_gc.is_some() {
                out.skipped.push(Skipped { id: tweak.id.into(), reason: "solo puede haber un recolector de memoria".into() });
                continue;
            }
            chosen_gc = Some(gc);
        }
        kept.push(tweak);
    }

    // Tercero: los que dependen de un recolector concreto.
    let effective_gc = if custom_has_unknown_gc { None } else { custom_gc.or(chosen_gc).or(Some(env.default_gc())) };
    for tweak in kept {
        if let Some(required) = tweak.requires_gc {
            if effective_gc != Some(required) {
                out.skipped.push(Skipped { id: tweak.id.into(), reason: "no aplica con el recolector de memoria en uso".into() });
                continue;
            }
        }
        // Un argumento propio de la misma bandera manda sobre el del ajuste.
        let flags: Vec<String> = (tweak.flags)(env)
            .into_iter()
            .filter(|f| !custom.iter().any(|c| key(c) == key(f) && !f.contains("UnlockExperimentalVMOptions")))
            .collect();
        if flags.is_empty() {
            out.skipped.push(Skipped { id: tweak.id.into(), reason: "tus argumentos ya definen esta opcion".into() });
            continue;
        }
        out.groups.push(Group { id: tweak.id.into(), flags, custom: false });
    }

    for arg in custom {
        out.groups.push(Group { id: arg.clone(), flags: vec![arg.clone()], custom: true });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(java_major: u32, cpus: usize, ram_mb: u64) -> Env {
        Env { java_major, cpus, ram_mb, heap_mb: 4096 }
    }

    fn ids(r: &Resolved) -> Vec<&str> {
        r.applied_ids()
    }

    #[test]
    fn applies_requested_tweaks_in_order_without_duplicates() {
        let r = resolve(&["g1", "no-explicit-gc", "g1", "no-perfdata"], &[], &env(21, 4, 8192));
        assert_eq!(ids(&r), ["g1", "no-explicit-gc", "no-perfdata"]);
        assert_eq!(r.flags(), ["-XX:+UseG1GC", "-XX:+DisableExplicitGC", "-XX:-UsePerfData"]);
        assert!(r.skipped.is_empty());
    }

    #[test]
    fn unknown_ids_are_reported_not_fatal() {
        let r = resolve(&["g1", "inventado"], &[], &env(17, 4, 8192));
        assert_eq!(ids(&r), ["g1"]);
        assert_eq!(r.skipped[0].id, "inventado");
    }

    #[test]
    fn zgc_needs_java_21_and_enough_ram_and_changes_flags_with_the_version() {
        assert!(resolve(&["zgc"], &[], &env(17, 8, 16384)).skipped[0].reason.contains("Java 21"));
        assert!(resolve(&["zgc"], &[], &env(21, 8, 4096)).skipped[0].reason.contains("RAM"));
        assert_eq!(resolve(&["zgc"], &[], &env(21, 8, 16384)).flags(), ["-XX:+UseZGC", "-XX:+ZGenerational"]);
        assert_eq!(resolve(&["zgc"], &[], &env(25, 8, 16384)).flags(), ["-XX:+UseZGC"]);
    }

    #[test]
    fn only_one_collector_survives() {
        let r = resolve(&["g1", "serial-gc"], &[], &env(21, 4, 8192));
        assert_eq!(ids(&r), ["g1"]);
        assert!(r.skipped[0].reason.contains("un recolector"));
    }

    #[test]
    fn custom_collector_beats_tweaks_and_drops_dependants() {
        let custom = vec!["-XX:+UseParallelGC".to_string()];
        let r = resolve(&["g1", "parallel-ref", "no-explicit-gc"], &custom, &env(21, 4, 8192));
        assert_eq!(ids(&r), ["no-explicit-gc"]);
        assert_eq!(r.skipped.len(), 2);
        assert_eq!(r.flags(), ["-XX:+DisableExplicitGC", "-XX:+UseParallelGC"]);
    }

    #[test]
    fn g1_dependants_work_on_the_default_collector() {
        // Java 17 con 4 CPU y 8 GB: la JVM ya usa G1, asi que `parallel-ref` sirve sin pedir `g1`.
        assert_eq!(ids(&resolve(&["parallel-ref"], &[], &env(17, 4, 8192))), ["parallel-ref"]);
        // Java 8 usa Parallel por defecto: no sirve.
        assert!(ids(&resolve(&["parallel-ref"], &[], &env(8, 4, 8192))).is_empty());
        // Un equipo de 1 CPU usa Serial.
        assert!(ids(&resolve(&["string-dedup"], &[], &env(17, 1, 8192))).is_empty());
    }

    #[test]
    fn custom_flag_overrides_the_same_tweak_flag() {
        let custom = vec!["-XX:G1NewSizePercent=20".to_string()];
        let r = resolve(&["g1", "g1-young"], &custom, &env(21, 4, 8192));
        let flags = r.flags();
        assert!(flags.contains(&"-XX:G1NewSizePercent=20".to_string()));
        assert!(!flags.contains(&"-XX:G1NewSizePercent=30".to_string()));
        // El resto del ajuste sigue y la bandera de desbloqueo se conserva.
        assert!(flags.contains(&"-XX:+UnlockExperimentalVMOptions".to_string()));
        assert!(flags.contains(&"-XX:G1ReservePercent=20".to_string()));
    }

    #[test]
    fn unlock_flag_precedes_experimental_ones() {
        let flags = resolve(&["g1", "g1-young"], &[], &env(21, 4, 8192)).flags();
        let unlock = flags.iter().position(|f| f == "-XX:+UnlockExperimentalVMOptions").unwrap();
        let young = flags.iter().position(|f| f.starts_with("-XX:G1NewSizePercent")).unwrap();
        assert!(unlock < young);
    }

    #[test]
    fn region_size_depends_on_heap() {
        let small = Env { heap_mb: 2048, ..env(21, 4, 8192) };
        assert!(!resolve(&["g1-young"], &[], &small).flags().iter().any(|f| f.contains("G1HeapRegionSize")));
        assert!(resolve(&["g1-young"], &[], &env(21, 4, 8192)).flags().iter().any(|f| f == "-XX:G1HeapRegionSize=8M"));
    }

    #[test]
    fn default_collector_follows_hotspot_ergonomics() {
        assert_eq!(env(21, 4, 8192).default_gc(), Gc::G1);
        assert_eq!(env(8, 4, 8192).default_gc(), Gc::Parallel);
        assert_eq!(env(21, 1, 8192).default_gc(), Gc::Serial);
        assert_eq!(env(21, 4, 1024).default_gc(), Gc::Serial);
    }
}
