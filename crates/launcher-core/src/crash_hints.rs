//! Explica en palabras simples por que se cerro el juego, a partir de lo que imprimio.
//!
//! Los modpacks fallan casi siempre por las mismas razones (Java que no encaja, poca memoria, mods que no
//! se llevan bien). En vez de mostrar «el juego se cerro» sin mas, se reconoce la firma del error y se
//! ofrece la accion que suele arreglarlo. Solo se usa cuando el juego termino con error.

use mods::ModConflict;

/// Causa conocida, de la mas a la menos especifica (la de menor numero gana si se ven varias).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CrashHint {
    JavaTooNew,
    JavaTooOld,
    HeapReserve,
    OutOfMemory,
    /// Dos mods que modifican el juego por debajo (cargadores) no son compatibles entre si.
    ModLoaderClash,
    IncompatibleMods,
}

impl CrashHint {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::JavaTooNew => {
                "El juego se cerro porque esta instancia no es compatible con el Java que se uso. \
                 Elige el Java automatico, o uno mas antiguo como Java 17, en los ajustes de Java de la instancia."
            }
            Self::JavaTooOld => {
                "El juego o uno de sus mods necesita una version de Java mas nueva. \
                 Elige el Java automatico en los ajustes de Java de la instancia."
            }
            Self::HeapReserve => {
                "El juego pidio mas memoria de la que tu equipo puede darle. \
                 Baja la memoria de la instancia en sus ajustes."
            }
            Self::OutOfMemory => {
                "El juego se quedo sin memoria. Sube la memoria de la instancia en sus ajustes \
                 (o quita mods pesados)."
            }
            Self::ModLoaderClash => "El juego se cerro porque dos mods no se llevan bien entre si.",
            Self::IncompatibleMods => {
                "Algunos mods no son compatibles entre si o con esta version del juego. \
                 Quita el ultimo mod que agregaste o desactiva los mods avisados en la consola."
            }
        }
    }
}

/// Reconoce la causa en una linea de salida del juego.
pub(crate) fn classify_line(line: &str) -> Option<CrashHint> {
    if line.contains("Unsupported class file major version") {
        return Some(CrashHint::JavaTooNew);
    }
    if line.contains("UnsupportedClassVersionError") || line.contains("compiled by a more recent version of the Java Runtime") {
        return Some(CrashHint::JavaTooOld);
    }
    if line.contains("Could not reserve enough space") || line.contains("Invalid maximum heap size") {
        return Some(CrashHint::HeapReserve);
    }
    // Solo la excepcion que tumba el hilo principal: las capturadas e impresas por mods no son un cierre por memoria.
    if line.contains("Exception in thread") && line.contains("java.lang.OutOfMemoryError") {
        return Some(CrashHint::OutOfMemory);
    }
    // Un mod llama a una pieza del cargador de otro mod que no existe en la version que hay cargada.
    if line.contains("NoSuchMethodError") && line.contains("org.spongepowered.asm.mixin.transformer.IMixinTransformer") {
        return Some(CrashHint::ModLoaderClash);
    }
    const MOD_SIGNATURES: &[&str] = &[
        "Incompatible mod set",
        "Incompatible mods found",
        "Mod resolution failed",
        "Missing or unsupported mandatory dependencies",
        "net.minecraftforge.fml.ModLoadingException",
        "net.neoforged.fml.ModLoadingException",
        "MixinTransformerError",
    ];
    MOD_SIGNATURES.iter().any(|s| line.contains(s)).then_some(CrashHint::IncompatibleMods)
}

/// Mensaje para el usuario cuando el juego se cerro con error: la causa reconocida, los choques entre
/// mods ya detectados y los mods que salen en la traza. `None` si no hay nada concreto que decir.
pub(crate) fn explain(hint: Option<CrashHint>, conflicts: &[ModConflict], suspects: &[String]) -> Option<String> {
    // Un Java o una memoria inadecuados se explican solos; los mods solo importan si el fallo pudo ser suyo.
    let mods_may_be_to_blame = matches!(hint, None | Some(CrashHint::ModLoaderClash | CrashHint::IncompatibleMods));
    let conflicts: &[ModConflict] = if mods_may_be_to_blame { conflicts } else { &[] };
    let suspects: &[String] = if mods_may_be_to_blame { suspects } else { &[] };
    if hint.is_none() && conflicts.is_empty() && suspects.is_empty() {
        return None;
    }
    // Con los mods ya nombrados, la frase generica de «dos mods no se llevan bien» sobra.
    let base = match hint {
        Some(CrashHint::ModLoaderClash) if !conflicts.is_empty() => "Minecraft se cerro por un choque entre mods.",
        Some(known) => known.message(),
        None => "Minecraft se cerro con un problema.",
    };
    let mut text = base.to_string();
    if !conflicts.is_empty() {
        let sentences: Vec<String> = conflicts.iter().take(2).map(ModConflict::sentence).collect();
        text.push_str(&format!(" {}.", sentences.join(". ")));
    } else if !suspects.is_empty() {
        text.push_str(&format!(" Mods implicados: {}.", suspects.join(", ")));
    }
    if !conflicts.is_empty() || !suspects.is_empty() {
        text.push_str(" Desactiva o quita en Mods el ultimo que agregaste.");
    }
    Some(text)
}

/// Archivo `.jar` que aparece en una linea de traza de pila (`at a.B.c(B.java:1) ~[mod-1.0.jar:?]`).
pub(crate) fn jar_in_frame(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("at ") {
        return None;
    }
    let end = line.rfind(".jar:")? + ".jar".len();
    let start = line[..end].rfind('[')? + 1;
    let name = line[start..end].trim_start_matches('~');
    (!name.is_empty() && !name.contains(['/', '\\'])).then_some(name)
}

/// Acumula la causa mas especifica vista durante la partida y los archivos que salen en las trazas de error.
#[derive(Default)]
pub(crate) struct CrashWatch {
    hint: Option<CrashHint>,
    /// (archivo, veces visto), en orden de aparicion.
    jars: Vec<(String, u32)>,
}

impl CrashWatch {
    /// Tope de archivos distintos recordados: una traza de mods no suele pasar de unas decenas.
    const MAX_JARS: usize = 64;

    pub(crate) fn observe(&mut self, line: &str) {
        if let Some(hint) = classify_line(line) {
            self.hint = Some(self.hint.map_or(hint, |current| current.min(hint)));
        }
        if let Some(jar) = jar_in_frame(line) {
            if let Some((_, seen)) = self.jars.iter_mut().find(|(name, _)| name.eq_ignore_ascii_case(jar)) {
                *seen += 1;
            } else if self.jars.len() < Self::MAX_JARS {
                self.jars.push((jar.to_string(), 1));
            }
        }
    }

    pub(crate) fn hint(&self) -> Option<CrashHint> {
        self.hint
    }

    /// Archivos de la traza, los mas repetidos primero.
    pub(crate) fn jars(&self) -> Vec<&str> {
        let mut ranked: Vec<&(String, u32)> = self.jars.iter().collect();
        ranked.sort_by_key(|(_, seen)| std::cmp::Reverse(*seen));
        ranked.into_iter().map(|(name, _)| name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_usual_modpack_failures() {
        let cases = [
            ("IllegalArgumentException: Unsupported class file major version 65", CrashHint::JavaTooNew),
            ("java.lang.UnsupportedClassVersionError: X has been compiled by a more recent version", CrashHint::JavaTooOld),
            ("Error occurred during initialization of VM\nCould not reserve enough space for object heap", CrashHint::HeapReserve),
            ("Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space", CrashHint::OutOfMemory),
            ("Caused by: java.lang.NoSuchMethodError: org.spongepowered.asm.mixin.transformer.IMixinTransformer.couldTransformClass(Lorg/spongepowered/asm/mixin/MixinEnvironment;Ljava/lang/String;)Z", CrashHint::ModLoaderClash),
            ("net.fabricmc.loader.impl.FormattedException: Incompatible mod set!", CrashHint::IncompatibleMods),
            ("net.minecraftforge.fml.ModLoadingException: Missing or unsupported mandatory dependencies", CrashHint::IncompatibleMods),
        ];
        for (line, expected) in cases {
            assert_eq!(classify_line(line), Some(expected), "{line}");
        }
        assert_eq!(classify_line("[main/INFO]: Loading Minecraft 1.19 with Fabric Loader 0.14.8"), None);
    }

    #[test]
    fn the_most_specific_cause_wins_over_its_symptoms() {
        // Un Java demasiado nuevo termina como «Mixin transformation failed»: la causa real es el Java.
        let mut watch = CrashWatch::default();
        watch.observe("org.spongepowered.asm.mixin.transformer.throwables.MixinTransformerError: unexpected");
        watch.observe("Error loading class: java/lang/Thread (IllegalArgumentException: Unsupported class file major version 65)");
        assert_eq!(watch.hint(), Some(CrashHint::JavaTooNew));
    }

    #[test]
    fn explanation_names_the_mods_only_when_they_may_be_to_blame() {
        let clash = ModConflict { first: "Essential".into(), second: "MixinBooter".into(), reason: "no encajan".into() };
        let suspects = vec!["Alfa".to_string()];
        let with_clash = explain(Some(CrashHint::ModLoaderClash), std::slice::from_ref(&clash), &suspects).unwrap();
        assert!(with_clash.contains("Essential y MixinBooter no se llevan bien"), "{with_clash}");
        assert!(!with_clash.contains("Alfa"), "con un choque conocido no se listan sospechosos: {with_clash}");

        let unknown = explain(None, &[], &suspects).unwrap();
        assert!(unknown.contains("Mods implicados: Alfa"), "{unknown}");

        assert_eq!(explain(None, &[], &[]), None);
        let java = explain(Some(CrashHint::JavaTooNew), &[clash], &suspects).unwrap();
        assert!(!java.contains("Essential") && !java.contains("Alfa"), "un Java inadecuado no culpa a los mods: {java}");
    }

    #[test]
    fn finds_the_jars_named_in_stack_frames() {
        let line = "	at zone.rong.mixinbooter.MixinBooterPlugin.initialize(MixinBooterPlugin.java:100) ~[!mixinbooter-12.2.jar:?]";
        assert_eq!(jar_in_frame(line), Some("!mixinbooter-12.2.jar"));
        assert_eq!(jar_in_frame("	at a.B.c(B.java:1) [forge-1.12.2-14.23.5.2859.jar:?]"), Some("forge-1.12.2-14.23.5.2859.jar"));
        assert_eq!(jar_in_frame("[main/INFO] [FML]: Searching mods in X.jar: found"), None, "solo trazas de pila");
        assert_eq!(jar_in_frame("	at a.B.c(B.java:1)"), None);
    }

    #[test]
    fn explains_the_real_manguero_crash_from_its_log() {
        // Fragmento del latest.log real: Essential relanza el juego y MixinBooter falla al cargarse.
        let log = [
            "[21:08:01] [main/ERROR] [LaunchWrapper]: Unable to launch",
            "java.lang.RuntimeException: An error occurred trying to configure the Minecraft home",
            "\tat net.minecraftforge.fml.relauncher.FMLLaunchHandler.setupHome(FMLLaunchHandler.java:111) ~[forge-1.12.2-14.23.5.2859.jar:?]",
            "Caused by: java.lang.NoSuchMethodError: org.spongepowered.asm.mixin.transformer.IMixinTransformer.couldTransformClass(Lorg/spongepowered/asm/mixin/MixinEnvironment;Ljava/lang/String;)Z",
            "\tat zone.rong.mixinbooter.MixinBooterPlugin.initialize(MixinBooterPlugin.java:100) ~[!mixinbooter-12.2.jar:?]",
            "\tat gg.essential.loader.stage2.Loader.relaunch(Loader.java:141) [essential-loader-stage2-4148095917507612197.jar:?]",
            "\tat gg.essential.loader.stage0.EssentialSetupTweaker.loadStage1(EssentialSetupTweaker.java:53) ~[Essential_1-5-1_forge_1-12-2.jar:?]",
        ];
        let mut watch = CrashWatch::default();
        for line in log {
            watch.observe(line);
        }
        assert_eq!(watch.hint(), Some(CrashHint::ModLoaderClash));
        assert!(watch.jars().contains(&"!mixinbooter-12.2.jar") && watch.jars().contains(&"Essential_1-5-1_forge_1-12-2.jar"));
    }

    #[test]
    fn ranks_the_most_repeated_jars_first() {
        let mut watch = CrashWatch::default();
        for line in ["	at a.A.x(A.java:1) ~[uno.jar:?]", "	at b.B.x(B.java:1) ~[dos.jar:?]", "	at b.B.y(B.java:2) ~[DOS.jar:?]"] {
            watch.observe(line);
        }
        assert_eq!(watch.jars(), vec!["dos.jar", "uno.jar"]);
    }

    #[test]
    fn messages_are_plain_spanish_without_internal_terms() {
        for hint in [CrashHint::JavaTooNew, CrashHint::JavaTooOld, CrashHint::HeapReserve, CrashHint::OutOfMemory, CrashHint::ModLoaderClash, CrashHint::IncompatibleMods] {
            let text = hint.message();
            assert!(!text.contains("Mixin") && !text.contains("JVM") && !text.contains("heap"), "{text}");
        }
    }
}
