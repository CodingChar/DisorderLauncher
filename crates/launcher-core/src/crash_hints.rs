//! Explica en palabras simples por que se cerro el juego, a partir de lo que imprimio.
//!
//! Los modpacks fallan casi siempre por las mismas razones (Java que no encaja, poca memoria, mods que no
//! se llevan bien). En vez de mostrar «el juego se cerro» sin mas, se reconoce la firma del error y se
//! ofrece la accion que suele arreglarlo. Solo se usa cuando el juego termino con error.

/// Causa conocida, de la mas a la menos especifica (la de menor numero gana si se ven varias).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CrashHint {
    JavaTooNew,
    JavaTooOld,
    HeapReserve,
    OutOfMemory,
    /// Un mod que relanza el juego con su propio cargador (p. ej. Essential) choca con otro mod.
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
            Self::ModLoaderClash => {
                "El juego se cerro porque el mod Essential no se lleva bien con otro mod de la instancia                  (como el que instalan Entity Culling o Distant Horizons). Quita Essential en Mods y vuelve a jugar."
            }
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
    // Essential relanza el juego con su propio sistema de carga; MixinBooter 10+ no lo entiende.
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

/// Acumula la causa mas especifica vista durante la partida.
#[derive(Default)]
pub(crate) struct CrashWatch(Option<CrashHint>);

impl CrashWatch {
    pub(crate) fn observe(&mut self, line: &str) {
        if let Some(hint) = classify_line(line) {
            self.0 = Some(self.0.map_or(hint, |current| current.min(hint)));
        }
    }

    pub(crate) fn hint(&self) -> Option<CrashHint> {
        self.0
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
    fn messages_are_plain_spanish_without_internal_terms() {
        for hint in [CrashHint::JavaTooNew, CrashHint::JavaTooOld, CrashHint::HeapReserve, CrashHint::OutOfMemory, CrashHint::ModLoaderClash, CrashHint::IncompatibleMods] {
            let text = hint.message();
            assert!(!text.contains("Mixin") && !text.contains("JVM") && !text.contains("heap"), "{text}");
        }
    }
}
