//! Catalogo de ajustes de la JVM: cada uno declara que banderas aporta, con que Java y equipos tiene sentido
//! y de donde sale la recomendacion. Anadir un ajuste es anadir una entrada aqui; nada mas necesita cambiar.

use serde::Serialize;

/// Recolectores de basura que un ajuste puede elegir o exigir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Gc {
    Serial,
    Parallel,
    G1,
    Zgc,
}

/// Cuanto se aparta un ajuste de los valores de la JVM; la interfaz lo muestra para que se elija con criterio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Risk {
    /// Sin contrapartida apreciable.
    Safe,
    /// Mejora una cosa a cambio de otra (mas memoria, arranque mas lento...).
    Tradeoff,
    /// Opciones experimentales de HotSpot: pueden cambiar entre versiones de Java.
    Experimental,
}

/// Lo que se sabe del equipo y del lanzamiento al decidir las banderas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Env {
    /// Version principal del Java que se va a usar (8, 17, 21...).
    pub java_major: u32,
    /// Hilos logicos del procesador.
    pub cpus: usize,
    /// Memoria fisica del equipo en MB.
    pub ram_mb: u64,
    /// Heap maximo que tendra el juego en MB.
    pub heap_mb: u64,
}

impl Env {
    /// Recolector que elegiria la JVM por si sola (ergonomia de HotSpot): Parallel en Java 8, G1 desde Java 9
    /// en equipos de servidor (2+ CPU y 2+ GB) y Serial en el resto.
    pub fn default_gc(&self) -> Gc {
        if self.cpus >= 2 && self.ram_mb >= 1792 {
            if self.java_major >= 9 { Gc::G1 } else { Gc::Parallel }
        } else {
            Gc::Serial
        }
    }
}

pub struct Tweak {
    pub id: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    /// Recolector que este ajuste selecciona, si es de los que lo hacen.
    pub selects_gc: Option<Gc>,
    /// Solo tiene efecto con este recolector.
    pub requires_gc: Option<Gc>,
    pub min_java: u32,
    pub max_java: Option<u32>,
    /// Memoria fisica minima del equipo para que merezca la pena.
    pub min_ram_mb: u64,
    pub risk: Risk,
    /// Documentacion o fuente que respalda el ajuste.
    pub source: &'static str,
    pub flags: fn(&Env) -> Vec<String>,
}

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const ORACLE_JAVA: &str = "https://docs.oracle.com/en/java/javase/21/docs/specs/man/java.html";
const AIKAR: &str = "https://docs.papermc.io/paper/aikars-flags";
const JEP_439: &str = "https://openjdk.org/jeps/439";

pub static CATALOG: &[Tweak] = &[
    Tweak {
        id: "g1",
        label: "Recolector G1",
        hint: "Recolector de basura con pausas cortas. Es el de Java 9 o superior; en Java 8 hay que pedirlo.",
        selects_gc: Some(Gc::G1),
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Safe,
        source: ORACLE_JAVA,
        flags: |_| strs(&["-XX:+UseG1GC"]),
    },
    Tweak {
        id: "serial-gc",
        label: "Recolector simple",
        hint: "Un solo hilo de limpieza de memoria: pensado para procesadores de 1 o 2 nucleos o poca RAM.",
        selects_gc: Some(Gc::Serial),
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Safe,
        source: ORACLE_JAVA,
        flags: |_| strs(&["-XX:+UseSerialGC"]),
    },
    Tweak {
        id: "zgc",
        label: "ZGC generacional",
        hint: "Pausas casi nulas a cambio de algo mas de memoria. Solo Java 21 o superior y equipos con 8 GB o mas.",
        selects_gc: Some(Gc::Zgc),
        requires_gc: None,
        min_java: 21,
        max_java: None,
        min_ram_mb: 8192,
        risk: Risk::Experimental,
        source: JEP_439,
        // JDK 21-22 necesita `-XX:+ZGenerational`; desde JDK 23 es el modo por defecto (JEP 474) y la bandera
        // queda obsoleta (JEP 490), asi que ya no se pasa.
        flags: |env| if env.java_major <= 22 { strs(&["-XX:+UseZGC", "-XX:+ZGenerational"]) } else { strs(&["-XX:+UseZGC"]) },
    },
    Tweak {
        id: "parallel-ref",
        label: "Limpieza en paralelo",
        hint: "Procesa referencias en varios hilos durante la limpieza de memoria: pausas algo mas cortas.",
        selects_gc: None,
        requires_gc: Some(Gc::G1),
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Safe,
        source: AIKAR,
        flags: |_| strs(&["-XX:+ParallelRefProcEnabled"]),
    },
    Tweak {
        id: "no-explicit-gc",
        label: "Ignorar limpiezas forzadas",
        hint: "Evita que un mod fuerce una limpieza completa de memoria y provoque un tiron.",
        selects_gc: None,
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Safe,
        source: AIKAR,
        flags: |_| strs(&["-XX:+DisableExplicitGC"]),
    },
    Tweak {
        id: "no-perfdata",
        label: "Sin datos de monitorizacion",
        hint: "Deja de escribir archivos de monitorizacion de la JVM: menos acceso a disco al arrancar.",
        selects_gc: None,
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Safe,
        source: ORACLE_JAVA,
        flags: |_| strs(&["-XX:-UsePerfData"]),
    },
    Tweak {
        id: "g1-young",
        label: "Memoria joven ampliada (G1)",
        hint: "Minecraft crea muchos objetos temporales: reserva mas espacio para ellos. Necesita 6 GB de RAM o mas.",
        selects_gc: None,
        requires_gc: Some(Gc::G1),
        min_java: 8,
        max_java: None,
        min_ram_mb: 6144,
        risk: Risk::Experimental,
        source: AIKAR,
        flags: |env| {
            let mut flags = strs(&["-XX:+UnlockExperimentalVMOptions", "-XX:G1NewSizePercent=30", "-XX:G1MaxNewSizePercent=40", "-XX:G1ReservePercent=20"]);
            // Regiones de 8 MB solo con heap suficiente: con uno pequeno quedarian muy pocas.
            if env.heap_mb >= 3072 {
                flags.push("-XX:G1HeapRegionSize=8M".into());
            }
            flags
        },
    },
    Tweak {
        id: "string-dedup",
        label: "Textos sin duplicar (G1)",
        hint: "Comparte textos repetidos en memoria: ahorra RAM con un pequeno coste de procesador.",
        selects_gc: None,
        requires_gc: Some(Gc::G1),
        min_java: 8,
        max_java: None,
        min_ram_mb: 0,
        risk: Risk::Tradeoff,
        source: ORACLE_JAVA,
        flags: |_| strs(&["-XX:+UseStringDeduplication"]),
    },
    Tweak {
        id: "pretouch",
        label: "Reservar la memoria al inicio",
        hint: "Prepara toda la memoria al arrancar: menos tirones despues, pero el inicio tarda mas y usa toda la RAM asignada.",
        selects_gc: None,
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 8192,
        risk: Risk::Tradeoff,
        source: AIKAR,
        flags: |_| strs(&["-XX:+AlwaysPreTouch"]),
    },
    Tweak {
        id: "code-cache",
        label: "Cache de codigo ampliada",
        hint: "Para modpacks grandes: evita que la JVM deje de optimizar cuando se llena su cache (240 MB por defecto).",
        selects_gc: None,
        requires_gc: None,
        min_java: 8,
        max_java: None,
        min_ram_mb: 6144,
        risk: Risk::Safe,
        source: ORACLE_JAVA,
        flags: |_| strs(&["-XX:ReservedCodeCacheSize=512m"]),
    },
];

pub fn find(id: &str) -> Option<&'static Tweak> {
    CATALOG.iter().find(|t| t.id == id)
}
