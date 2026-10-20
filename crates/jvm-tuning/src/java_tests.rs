//! Pruebas contra Javas reales: demuestran que cada ajuste del catalogo lo acepta la JVM y que surte el efecto
//! que promete (se lee el valor final con `-XX:+PrintFlagsFinal`).
//!
//! Javas a probar: `DISORDER_TEST_JAVAS` (rutas a `java`/`java.exe` separadas por `;`) o, si no existe, `JAVA_HOME`.
//! Sin ningun Java las pruebas se saltan avisando.

use crate::*;
use std::path::PathBuf;
use std::process::Command;

fn candidates() -> Vec<PathBuf> {
    if let Ok(list) = std::env::var("DISORDER_TEST_JAVAS") {
        return list.split(';').filter(|s| !s.trim().is_empty()).map(PathBuf::from).collect();
    }
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    std::env::var_os("JAVA_HOME").map(|home| vec![PathBuf::from(home).join("bin").join(exe)]).unwrap_or_default()
}

fn major_of(java: &PathBuf) -> u32 {
    let output = Command::new(java).arg("-version").output().expect("ejecutar java");
    let text = String::from_utf8_lossy(&output.stderr).into_owned();
    let version = text.split('"').nth(1).unwrap_or("0");
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    if first == 1 { parts.next().and_then(|p| p.parse().ok()).unwrap_or(0) } else { first }
}

fn env_for(java_major: u32) -> Env {
    Env { java_major, cpus: 4, ram_mb: 16384, heap_mb: 4096 }
}

#[tokio::test]
async fn every_applicable_tweak_is_accepted_by_each_real_java() {
    let javas = candidates();
    if javas.is_empty() {
        eprintln!("sin Java de prueba: defina DISORDER_TEST_JAVAS o JAVA_HOME");
        return;
    }
    for java in javas {
        let major = major_of(&java);
        let env = env_for(major);
        for tweak in CATALOG {
            let resolved = resolve(&["g1", tweak.id], &[], &env);
            let applied = resolved.applied_ids().contains(&tweak.id);
            if !applied {
                // Debe ser por una razon declarada (version, RAM o recolector), nunca silenciosa.
                assert!(resolved.skipped.iter().any(|s| s.id == tweak.id) || tweak.selects_gc.is_some(), "{} sin motivo en Java {major}", tweak.id);
                continue;
            }
            let flags = resolved.flags();
            probe(&java, &flags).await.unwrap_or_else(|e| panic!("Java {major} rechaza «{}» ({flags:?}): {e}", tweak.id));
        }
    }
}

#[tokio::test]
async fn flags_take_effect_in_the_real_jvm() {
    let javas = candidates();
    if javas.is_empty() {
        eprintln!("sin Java de prueba: defina DISORDER_TEST_JAVAS o JAVA_HOME");
        return;
    }
    for java in javas {
        let major = major_of(&java);
        let env = env_for(major);
        let check = |ids: &'static [&'static str]| {
            let flags = resolve(ids, &[], &env).flags();
            (flags, ids)
        };

        let (flags, _) = check(&["g1", "no-explicit-gc", "no-perfdata", "parallel-ref", "string-dedup", "code-cache", "pretouch"]);
        let value = |name: &'static str| {
            let (java, flags) = (java.clone(), flags.clone());
            async move { final_value(&java, &flags, name).await }
        };
        assert_eq!(value("UseG1GC").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(value("DisableExplicitGC").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(value("UsePerfData").await.as_deref(), Some("false"), "Java {major}");
        assert_eq!(value("ParallelRefProcEnabled").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(value("UseStringDeduplication").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(value("AlwaysPreTouch").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(value("ReservedCodeCacheSize").await.as_deref(), Some("536870912"), "Java {major}");

        let (flags, _) = check(&["g1", "g1-young"]);
        assert_eq!(final_value(&java, &flags, "G1NewSizePercent").await.as_deref(), Some("30"), "Java {major}");
        assert_eq!(final_value(&java, &flags, "G1MaxNewSizePercent").await.as_deref(), Some("40"), "Java {major}");
        assert_eq!(final_value(&java, &flags, "G1ReservePercent").await.as_deref(), Some("20"), "Java {major}");
        assert_eq!(final_value(&java, &flags, "G1HeapRegionSize").await.as_deref(), Some("8388608"), "Java {major}");

        let (flags, _) = check(&["serial-gc"]);
        assert_eq!(final_value(&java, &flags, "UseSerialGC").await.as_deref(), Some("true"), "Java {major}");
        assert_eq!(final_value(&java, &flags, "UseG1GC").await.as_deref(), Some("false"), "Java {major}");

        if major >= 21 {
            let (flags, _) = check(&["zgc"]);
            assert_eq!(final_value(&java, &flags, "UseZGC").await.as_deref(), Some("true"), "Java {major}");
            if major <= 22 {
                assert_eq!(final_value(&java, &flags, "ZGenerational").await.as_deref(), Some("true"), "Java {major}");
            }
        }
    }
}

#[tokio::test]
async fn a_bad_custom_argument_is_dropped_and_the_rest_still_work() {
    let Some(java) = candidates().into_iter().next() else {
        eprintln!("sin Java de prueba");
        return;
    };
    let env = env_for(major_of(&java));
    let custom = parse_args("-XX:+EstaOpcionNoExiste -XX:MaxGCPauseMillis=150").unwrap();
    let resolved = resolve(&["g1", "no-explicit-gc"], &custom, &env);
    // La propia JVM las rechaza todas juntas...
    assert!(probe(&java, &resolved.flags()).await.is_err());
    // ...y `keep_working` aparta solo la mala.
    let working = keep_working(&java, resolved).await;
    let flags = working.flags();
    assert!(!flags.contains(&"-XX:+EstaOpcionNoExiste".to_string()));
    assert!(flags.contains(&"-XX:MaxGCPauseMillis=150".to_string()));
    assert!(flags.contains(&"-XX:+UseG1GC".to_string()));
    assert_eq!(working.skipped.len(), 1);
    assert!(working.skipped[0].reason.contains("Java no la acepta"));
    assert_eq!(final_value(&java, &flags, "MaxGCPauseMillis").await.as_deref(), Some("150"));
}

#[tokio::test]
async fn experimental_flag_without_unlock_is_caught_by_the_probe() {
    let Some(java) = candidates().into_iter().next() else {
        return;
    };
    // Sin `-XX:+UnlockExperimentalVMOptions` la JVM no arranca: justo lo que la comprobacion evita en el juego.
    let err = probe(&java, &["-XX:G1NewSizePercent=30".to_string()]).await.unwrap_err();
    assert!(!err.is_empty());
}
