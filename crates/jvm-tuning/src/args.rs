//! Argumentos de Java escritos por la persona: se separan, se validan y se rechaza lo que pueda ejecutar
//! programas, cargar codigo o abrir puertos. Nunca pasan por un shell (el launcher lanza Java con argv directo),
//! asi que aqui solo importa lo que la propia JVM podria hacer con ellos.

use crate::catalog::Gc;

pub const MAX_ARGS: usize = 40;
pub const MAX_ARG_LEN: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgError {
    pub arg: String,
    pub reason: String,
}

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "«{}»: {}", self.arg, self.reason)
    }
}

/// Prefijos que se aceptan (el resto se rechaza): opciones de la JVM, propiedades del sistema y `-X`.
const ALLOWED_PREFIXES: &[&str] = &["-XX:", "-D", "-X"];

/// Opciones que ejecutan comandos, cargan codigo nativo o de otra ubicacion, escriben en rutas arbitrarias
/// o abren puertos de control. Se comparan sin distinguir mayusculas.
const DENIED: &[(&str, &str)] = &[
    ("-xx:onerror", "ejecuta un programa cuando la JVM falla"),
    ("-xx:onoutofmemoryerror", "ejecuta un programa cuando falta memoria"),
    ("-xx:flags", "lee opciones de un archivo"),
    ("-xx:vmoptionsfile", "lee opciones de un archivo"),
    ("-xx:errorfile", "escribe en una ruta arbitraria"),
    ("-xx:heapdumppath", "escribe en una ruta arbitraria"),
    ("-xx:logfile", "escribe en una ruta arbitraria"),
    ("-xx:sharedarchivefile", "carga un archivo de clases externo"),
    ("-xx:+createcoredumponcrash", "escribe volcados enormes en disco"),
    ("-xbootclasspath", "reemplaza las clases base de Java"),
    ("-xrun", "carga librerias nativas"),
    ("-xdebug", "abre el depurador"),
    ("-xlog:", "puede escribir registros en rutas arbitrarias"),
    ("-xshare:dump", "regenera archivos de la instalacion de Java"),
    ("-djava.library.path", "el launcher la gestiona"),
    ("-djava.class.path", "el launcher la gestiona"),
    ("-djava.ext.dirs", "carga codigo externo"),
    ("-djava.security", "cambia la seguridad de Java"),
    ("-djavax.net.ssl", "cambia la validacion de certificados"),
    ("-djdk.http", "cambia el comportamiento de red de Java"),
    ("-dcom.sun.management", "abre un puerto de control remoto"),
    ("-djava.rmi", "abre un puerto de control remoto"),
    ("-duser.home", "cambia la carpeta personal"),
    ("-duser.dir", "cambia la carpeta de trabajo"),
    ("-djna.tmpdir", "el launcher la gestiona"),
    ("-dminecraft.launcher", "el launcher la gestiona"),
];

/// Banderas que el launcher gestiona con otros ajustes.
const MANAGED: &[(&str, &str)] = &[
    ("-xmx", "la memoria se elige en «Opciones avanzadas > Memoria»"),
    ("-xms", "la memoria se elige en «Opciones avanzadas > Memoria»"),
    ("-xmn", "la memoria se elige en «Opciones avanzadas > Memoria»"),
];

/// Separa por espacios y valida cada argumento. No hay comillas: un argumento no puede contener espacios.
pub fn parse(input: &str) -> Result<Vec<String>, ArgError> {
    let args: Vec<String> = input.split_whitespace().map(str::to_string).collect();
    if args.len() > MAX_ARGS {
        return Err(ArgError { arg: format!("{} argumentos", args.len()), reason: format!("hay demasiados (maximo {MAX_ARGS})") });
    }
    for arg in &args {
        validate_one(arg)?;
    }
    Ok(args)
}

fn validate_one(arg: &str) -> Result<(), ArgError> {
    let reject = |reason: &str| Err(ArgError { arg: arg.to_string(), reason: reason.to_string() });
    if arg.chars().count() > MAX_ARG_LEN {
        return reject("es demasiado largo");
    }
    if !arg.is_ascii() || arg.chars().any(|c| c.is_control()) {
        return reject("solo admite caracteres ASCII visibles");
    }
    if arg.starts_with('@') {
        return reject("no se admiten archivos de argumentos");
    }
    if !ALLOWED_PREFIXES.iter().any(|p| arg.starts_with(p)) {
        return reject("debe empezar por -XX:, -X o -D");
    }
    let lower = arg.to_ascii_lowercase();
    if let Some((_, why)) = MANAGED.iter().find(|(p, _)| lower.starts_with(p)) {
        return reject(why);
    }
    if let Some((_, why)) = DENIED.iter().find(|(p, _)| lower.starts_with(p)) {
        return reject(&format!("no se permite: {why}"));
    }
    // `-XX:` con y sin signo (`-XX:+Flag`, `-XX:-Flag`) comparte nombre: se mira el nombre pelado.
    let bare = lower.strip_prefix("-xx:").map(|r| r.trim_start_matches(['+', '-']));
    if let Some(name) = bare {
        if let Some((_, why)) = DENIED.iter().find(|(p, _)| p.strip_prefix("-xx:").is_some_and(|d| name.starts_with(d.trim_start_matches(['+', '-'])))) {
            return reject(&format!("no se permite: {why}"));
        }
    }
    Ok(())
}

/// Nombre con el que se compara una bandera para saber si dos se pisan (`-XX:+Foo`, `-XX:-Foo` y `-XX:Foo=3` son
/// la misma; `-Dclave=a` y `-Dclave=b` tambien).
pub fn key(flag: &str) -> String {
    if let Some(rest) = flag.strip_prefix("-XX:") {
        let rest = rest.trim_start_matches(['+', '-']);
        return format!("-XX:{}", rest.split('=').next().unwrap_or(rest));
    }
    if flag.starts_with("-D") {
        return flag.split('=').next().unwrap_or(flag).to_string();
    }
    flag.to_string()
}

/// Bandera de un recolector que este launcher no conoce (Shenandoah, CMS...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownGc;

/// Recolector que selecciona una bandera concreta (`-XX:+UseG1GC`...); `Err` si es de uno desconocido.
/// `Ok(None)`: no es un selector.
pub fn selected_gc(flag: &str) -> Result<Option<Gc>, UnknownGc> {
    match flag {
        "-XX:+UseSerialGC" => Ok(Some(Gc::Serial)),
        "-XX:+UseParallelGC" | "-XX:+UseParallelOldGC" => Ok(Some(Gc::Parallel)),
        "-XX:+UseG1GC" => Ok(Some(Gc::G1)),
        "-XX:+UseZGC" => Ok(Some(Gc::Zgc)),
        f if f.starts_with("-XX:+Use") && f.ends_with("GC") => Err(UnknownGc),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_regular_tuning_flags() {
        let args = parse("-XX:+UseG1GC -XX:MaxGCPauseMillis=100 -Dfile.encoding=UTF-8 -Xss2m").unwrap();
        assert_eq!(args.len(), 4);
        assert!(parse("").unwrap().is_empty());
        assert!(parse("   \t ").unwrap().is_empty());
    }

    #[test]
    fn rejects_flags_that_run_programs_or_load_code() {
        for bad in [
            "-XX:OnError=calc.exe",
            "-XX:OnOutOfMemoryError=cmd",
            "-XX:+OnError=x",
            "-javaagent:evil.jar",
            "-agentlib:jdwp=transport=dt_socket",
            "-agentpath:x.dll",
            "-Xbootclasspath/a:x.jar",
            "-Xrunjdwp:x",
            "-cp",
            "-jar",
            "@args.txt",
            "-XX:VMOptionsFile=x",
            "-XX:Flags=.hotspotrc",
            "-Djava.library.path=C:/x",
            "-Dcom.sun.management.jmxremote.port=9999",
            "-Djavax.net.ssl.trustStore=x",
            "-XX:ErrorFile=C:/Windows/x",
            "-Xlog:gc:file=C:/x.txt",
            "-XX:SharedArchiveFile=x.jsa",
        ] {
            assert!(parse(bad).is_err(), "debia rechazar {bad}");
        }
    }

    #[test]
    fn denied_names_are_rejected_in_any_case_and_with_signs() {
        assert!(parse("-xx:onerror=x").is_err());
        assert!(parse("-XX:-HeapDumpPath=x").is_err());
        assert!(parse("-XX:+ErrorFile=x").is_err());
    }

    #[test]
    fn memory_is_left_to_the_memory_setting() {
        let err = parse("-Xmx8G").unwrap_err();
        assert!(err.reason.contains("Memoria"));
        assert!(parse("-Xms1G").is_err());
    }

    #[test]
    fn rejects_non_ascii_oversized_and_too_many() {
        assert!(parse("-Dnombre=ñ").is_err());
        assert!(parse(&format!("-Dx={}", "a".repeat(250))).is_err());
        let many = (0..41).map(|i| format!("-Dk{i}=1")).collect::<Vec<_>>().join(" ");
        assert!(parse(&many).is_err());
    }

    #[test]
    fn keys_ignore_sign_and_value() {
        assert_eq!(key("-XX:+UseG1GC"), key("-XX:-UseG1GC"));
        assert_eq!(key("-XX:MaxGCPauseMillis=50"), key("-XX:MaxGCPauseMillis=200"));
        assert_eq!(key("-Dfoo=1"), key("-Dfoo=2"));
        assert_ne!(key("-Dfoo=1"), key("-Dbar=1"));
    }

    #[test]
    fn detects_collector_selectors() {
        assert_eq!(selected_gc("-XX:+UseG1GC"), Ok(Some(Gc::G1)));
        assert_eq!(selected_gc("-XX:+UseZGC"), Ok(Some(Gc::Zgc)));
        assert_eq!(selected_gc("-XX:+UseShenandoahGC"), Err(UnknownGc));
        assert_eq!(selected_gc("-XX:+UseStringDeduplication"), Ok(None));
    }
}
