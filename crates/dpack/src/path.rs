use crate::{DpackError, MANIFEST_NAME};
use std::path::{Component, Path};

const MAX_PATH_LEN: usize = 240;
const RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3",
    "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Ruta relativa dentro de la carpeta de juego. Rechaza lo que podria escapar de ella o confundir al
/// sistema de archivos: `..`, raiz, unidades, `\`, `:`, nombres reservados de Windows y la carpeta
/// interna del launcher.
pub fn validate_entry_path(path: &str) -> Result<(), DpackError> {
    let bad = || DpackError::UnsafePath(path.chars().take(80).collect());
    if path.is_empty() || path.len() > MAX_PATH_LEN || path.contains(['\\', ':', '\0']) || path.chars().any(char::is_control) {
        return Err(bad());
    }
    let lower = path.to_lowercase();
    if lower.starts_with(".disorder") || lower == MANIFEST_NAME {
        return Err(bad());
    }
    let p = Path::new(path);
    if !p.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(bad());
    }
    for segment in path.split('/') {
        let stem = segment.split('.').next().unwrap_or("").to_lowercase();
        if segment.is_empty()
            || segment.ends_with(['.', ' '])
            || RESERVED.contains(&stem.as_str())
            || segment.len() > 255
        {
            return Err(bad());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_game_paths() {
        for ok in ["mods/sodium.jar", "config/a/b.toml", "kubejs/server_scripts/x.js", "resourcepacks/Pack v1.zip"] {
            assert!(validate_entry_path(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_escapes_and_oddities() {
        for bad in [
            "",
            "../fuera.jar",
            "mods/../../x",
            "/abs.jar",
            "C:/x.jar",
            "mods\\a.jar",
            "mods//a.jar",
            "./a.jar",
            ".disorder/mods.json",
            ".DISORDER/x",
            "dpack-manifest.json",
            "mods/con.jar",
            "mods/NUL",
            "mods/a.jar.",
            "mods/a.jar ",
            "mods/a\nb.jar",
        ] {
            assert!(validate_entry_path(bad).is_err(), "{bad:?}");
        }
        assert!(validate_entry_path(&"a/".repeat(200)).is_err());
    }
}
