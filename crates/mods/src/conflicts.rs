//! Mods que no se llevan bien entre si, detectados antes de que el juego se cierre.
//!
//! Dos fuentes: las incompatibilidades que declara Modrinth al instalar (se guardan en el registro de
//! mods) y una tabla propia de choques conocidos que Modrinth no declara. Anadir un caso nuevo es
//! anadir una fila a `KNOWN_CONFLICTS`; nada mas cambia.

use crate::{LockedMod, ModEntry};
use serde::Serialize;
use std::collections::HashSet;

/// Dos mods activos que pueden cerrar el juego o romperlo si van juntos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModConflict {
    pub first: String,
    pub second: String,
    /// Frase corta y sin terminos internos que explica el choque.
    pub reason: String,
}

impl ModConflict {
    pub fn sentence(&self) -> String {
        format!("{} y {} no se llevan bien: {}", self.first, self.second, self.reason)
    }
}

/// Como reconocer un mod: por proyecto de Modrinth, por id del mod o por el prefijo de su archivo.
struct Matcher {
    projects: &'static [&'static str],
    ids: &'static [&'static str],
    file_prefixes: &'static [&'static str],
}

struct KnownConflict {
    first: (&'static str, Matcher),
    second: (&'static str, Matcher),
    /// Solo aplica a esta version de Minecraft (prefijo), o a todas si es `None`.
    minecraft: Option<&'static str>,
    reason: &'static str,
}

const KNOWN_CONFLICTS: &[KnownConflict] = &[
    // Essential relanza el juego con su propio cargador de Mixin; MixinBooter (que traen muchos mods
    // de 1.12.2, como Entity Culling) usa otro y el juego se cierra al arrancar.
    KnownConflict {
        first: ("Essential", Matcher { projects: &["k2ZPuTBm"], ids: &["essential"], file_prefixes: &["essential"] }),
        second: ("MixinBooter", Matcher { projects: &["G1ckZuWK"], ids: &["mixinbooter"], file_prefixes: &["mixinbooter", "!mixinbooter"] }),
        minecraft: Some("1.12"),
        reason: "Essential reinicia el juego con su propio cargador y el de MixinBooter no encaja",
    },
    // Ambos sustituyen el motor de dibujo del juego.
    KnownConflict {
        first: ("OptiFine", Matcher { projects: &[], ids: &["optifine"], file_prefixes: &["optifine"] }),
        second: ("Sodium", Matcher { projects: &["AANobbMI", "sPvKXHrb"], ids: &["sodium", "embeddium", "rubidium"], file_prefixes: &["sodium", "embeddium", "rubidium"] }),
        minecraft: None,
        reason: "los dos reemplazan el motor de dibujo del juego",
    },
];

/// Un mod activo tal como lo ven las reglas.
struct Candidate {
    name: String,
    project_id: Option<String>,
    id: Option<String>,
    file: String,
    incompatible: Vec<String>,
}

impl Candidate {
    fn matches(&self, matcher: &Matcher) -> bool {
        self.project_id.as_deref().is_some_and(|p| matcher.projects.contains(&p))
            || self.id.as_deref().is_some_and(|i| matcher.ids.iter().any(|m| i.eq_ignore_ascii_case(m)))
            || matcher.file_prefixes.iter().any(|p| self.file.starts_with(p))
    }
}

/// Busca choques entre los mods activos. `locked` es el registro de Modrinth de la instancia.
pub fn find_conflicts(entries: &[ModEntry], locked: &[LockedMod], minecraft_version: &str) -> Vec<ModConflict> {
    let active: Vec<Candidate> = entries
        .iter()
        .filter(|e| e.enabled)
        .map(|e| {
            let lock = locked.iter().find(|l| l.file == e.file_name);
            Candidate {
                name: lock.map(|l| l.title.clone()).unwrap_or_else(|| e.name.clone()),
                project_id: lock.map(|l| l.project_id.clone()),
                id: e.meta.as_ref().map(|m| m.id.clone()),
                file: e.file_name.to_lowercase(),
                incompatible: lock.map(|l| l.incompatible.clone()).unwrap_or_default(),
            }
        })
        .collect();

    let mut found = Vec::new();
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut push = |a: usize, b: usize, reason: &str, found: &mut Vec<ModConflict>| {
        let key = (a.min(b), a.max(b));
        if a != b && seen.insert(key) {
            found.push(ModConflict { first: active[key.0].name.clone(), second: active[key.1].name.clone(), reason: reason.to_string() });
        }
    };

    for rule in KNOWN_CONFLICTS {
        if rule.minecraft.is_some_and(|prefix| !minecraft_version.starts_with(prefix)) {
            continue;
        }
        for (a, _) in active.iter().enumerate().filter(|(_, c)| c.matches(&rule.first.1)) {
            for (b, _) in active.iter().enumerate().filter(|(_, c)| c.matches(&rule.second.1)) {
                push(a, b, rule.reason, &mut found);
            }
        }
    }
    for (a, candidate) in active.iter().enumerate() {
        for (b, other) in active.iter().enumerate() {
            if other.project_id.as_ref().is_some_and(|p| candidate.incompatible.contains(p)) {
                push(a, b, "el autor de uno de los dos avisa de que son incompatibles", &mut found);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn entry(file: &str, enabled: bool) -> ModEntry {
        ModEntry {
            file_name: file.into(),
            enabled,
            name: file.trim_end_matches(".jar").into(),
            version: None,
            description: None,
            size: 1,
            meta: None,
            source: None,
        }
    }

    fn lock(file: &str, project: &str, title: &str, incompatible: &[&str]) -> LockedMod {
        LockedMod {
            file: file.into(),
            project_id: project.into(),
            version_id: "v".into(),
            version_number: "1".into(),
            title: title.into(),
            sha1: "aa".into(),
            installed_at: Utc::now(),
            dependency_of: None,
            previous: None,
            incompatible: incompatible.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn flags_essential_with_mixinbooter_on_1_12() {
        let entries = [entry("Essential_1-5-1_forge_1-12-2.jar", true), entry("!mixinbooter-12.2.jar", true), entry("otro.jar", true)];
        let locked = [lock("Essential_1-5-1_forge_1-12-2.jar", "k2ZPuTBm", "Essential Mod", &[]), lock("!mixinbooter-12.2.jar", "G1ckZuWK", "MixinBooter", &[])];
        let found = find_conflicts(&entries, &locked, "1.12.2");
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].first.as_str(), found[0].second.as_str()), ("Essential Mod", "MixinBooter"));
        assert!(find_conflicts(&entries, &locked, "1.20.1").is_empty(), "la regla es de 1.12");
    }

    #[test]
    fn recognises_mods_without_a_modrinth_record_by_file_name() {
        let entries = [entry("OptiFine_1.20.1_HD_U_I6.jar", true), entry("sodium-fabric-0.5.jar", true)];
        assert_eq!(find_conflicts(&entries, &[], "1.20.1").len(), 1);
    }

    #[test]
    fn disabled_mods_never_conflict() {
        let entries = [entry("Essential_1-5-1_forge_1-12-2.jar.disabled", false), entry("!mixinbooter-12.2.jar", true)];
        assert!(find_conflicts(&entries, &[], "1.12.2").is_empty());
    }

    #[test]
    fn uses_the_incompatibilities_modrinth_declares() {
        let entries = [entry("a.jar", true), entry("b.jar", true), entry("c.jar", true)];
        let locked = [lock("a.jar", "AAA", "Alpha", &["BBB"]), lock("b.jar", "BBB", "Beta", &[]), lock("c.jar", "CCC", "Gamma", &[])];
        let found = find_conflicts(&entries, &locked, "1.20.1");
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].first.as_str(), found[0].second.as_str()), ("Alpha", "Beta"));
    }

    #[test]
    fn a_pair_is_reported_once_even_when_both_sources_agree() {
        let entries = [entry("Essential_x.jar", true), entry("mixinbooter-10.jar", true)];
        let locked = [lock("Essential_x.jar", "k2ZPuTBm", "Essential Mod", &["G1ckZuWK"]), lock("mixinbooter-10.jar", "G1ckZuWK", "MixinBooter", &[])];
        assert_eq!(find_conflicts(&entries, &locked, "1.12.2").len(), 1);
    }
}
