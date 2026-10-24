use minecraft_launch::MemorySettings;

const MIN_HEAP_MB: u64 = 512;
const MAX_HEAP_CAP_MB: u64 = 8192;

/// RAM automatica: un cuarto de la memoria del equipo (entre 2 y 8 GB), y la mitad en
/// equipos con menos de 4 GB para no dejar sin memoria al sistema.
pub fn auto_memory(total_mb: u64) -> MemorySettings {
    let max = if total_mb < 4096 {
        (total_mb / 2).max(1024)
    } else {
        (total_mb / 4).clamp(2048, MAX_HEAP_CAP_MB)
    };
    MemorySettings {
        min_mb: Some(MIN_HEAP_MB.min(max) as u32),
        max_mb: Some(max as u32),
    }
}

/// RAM automatica teniendo en cuenta cuantos mods carga la instancia. Un modpack de 130 mods necesita bastante
/// mas que el juego solo: se sube el maximo (unos 20 MB por mod sobre 2 GB) sin pasar del 60 % de la memoria del
/// equipo, para que el sistema y el navegador sigan respirando. Nunca baja de lo que daria `auto_memory`.
pub fn auto_memory_for_mods(total_mb: u64, mod_count: usize) -> MemorySettings {
    let base = auto_memory(total_mb);
    if mod_count < 20 || total_mb == 0 {
        return base;
    }
    let wanted = 2048 + mod_count as u64 * 20;
    let ceiling = (total_mb * 6 / 10).min(MAX_HEAP_CAP_MB);
    let max = wanted.min(ceiling).max(u64::from(base.max_mb.unwrap_or(0)));
    MemorySettings { min_mb: base.min_mb, max_mb: Some(max as u32) }
}

/// Mods (`.jar`) que hay en la carpeta `mods` de la instancia.
pub fn count_mods(game_dir: &std::path::Path) -> usize {
    std::fs::read_dir(game_dir.join("mods"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("jar")))
                .count()
        })
        .unwrap_or(0)
}

/// Memoria fisica total del equipo en MB (0 si no se puede determinar).
pub fn system_memory_mb() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.total_memory() / (1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_with_system_memory() {
        assert_eq!(auto_memory(2048).max_mb, Some(1024));
        assert_eq!(auto_memory(8192).max_mb, Some(2048));
        assert_eq!(auto_memory(16384).max_mb, Some(4096));
        assert_eq!(auto_memory(65536).max_mb, Some(8192));
    }

    #[test]
    fn big_modpacks_get_more_memory_without_starving_the_system() {
        // Sin mods o con pocos: igual que antes.
        assert_eq!(auto_memory_for_mods(16384, 0).max_mb, auto_memory(16384).max_mb);
        assert_eq!(auto_memory_for_mods(16384, 10).max_mb, auto_memory(16384).max_mb);
        // 135 mods en un equipo de 10 GB: 2048 + 135*20 = 4748 MB (el 60 % serian 6144).
        assert_eq!(auto_memory_for_mods(10240, 135).max_mb, Some(4748));
        // En 6 GB el 60 % (3686) manda sobre lo deseado.
        assert_eq!(auto_memory_for_mods(6144, 135).max_mb, Some(3686));
        // Nunca menos que la memoria automatica ni mas de 8 GB.
        assert!(auto_memory_for_mods(4096, 135).max_mb >= auto_memory(4096).max_mb);
        assert_eq!(auto_memory_for_mods(65536, 400).max_mb, Some(8192));
        assert_eq!(auto_memory_for_mods(0, 135).max_mb, auto_memory(0).max_mb);
    }

    #[test]
    fn mods_are_counted_by_jar_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mods")).unwrap();
        for name in ["a.jar", "B.JAR", "notas.txt", "c.jar.disabled"] {
            std::fs::write(dir.path().join("mods").join(name), b"").unwrap();
        }
        assert_eq!(count_mods(dir.path()), 2);
        assert_eq!(count_mods(&dir.path().join("no-existe")), 0);
    }

    #[test]
    fn unknown_memory_still_gives_a_safe_value() {
        let m = auto_memory(0);
        assert_eq!(m.max_mb, Some(1024));
        assert!(m.min_mb <= m.max_mb);
    }

    #[test]
    fn system_memory_is_detected() {
        assert!(system_memory_mb() > 0);
    }
}
