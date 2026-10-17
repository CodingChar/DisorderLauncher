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
