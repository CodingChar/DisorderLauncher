//! Crash Assistant (un mod que traen muchos modpacks) muestra, tras cada crash, un aviso de
//! «launcher no soportado o modo offline» y se niega a ayudar. Es una opcion del propio mod
//! (`[piracy] enabled` en su `config.toml`); con un perfil Offline Gaming, que es el modo principal de
//! este launcher, el aviso solo estorba. Se desactiva con la opcion que el mod ofrece, sin tocar el jar.

use std::io;
use std::path::Path;

const CONFIG: &str = "config/crash_assistant/config.toml";

/// Pone `enabled = false` en la seccion `[piracy]` del `config.toml` de Crash Assistant, si existe y esta activa.
/// Devuelve `true` si cambio algo. Conserva comentarios, sangrado y saltos de linea del archivo.
pub(crate) fn silence_offline_notice(game_dir: &Path) -> io::Result<bool> {
    let path = game_dir.join(CONFIG);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    let Some(updated) = disable_piracy_notice(&text) else { return Ok(false) };
    let part = path.with_extension("toml.part");
    std::fs::write(&part, updated)?;
    std::fs::rename(&part, &path)?;
    Ok(true)
}

/// `None` si no hay nada que cambiar (la seccion no existe o ya esta desactivada).
fn disable_piracy_notice(text: &str) -> Option<String> {
    let mut in_section = false;
    let mut changed = false;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed.trim_end_matches(['\r', '\n']).trim() == "[piracy]";
        }
        if in_section && !changed {
            if let Some((key, value)) = trimmed.split_once('=') {
                if key.trim() == "enabled" && value.trim() == "true" {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    let newline = &line[line.trim_end_matches(['\r', '\n']).len()..];
                    out.push_str(&format!("{indent}enabled = false{newline}"));
                    changed = true;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    changed.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "[general]\n\tenabled = true\n\n#Settings of notifying about piracy/offline mode.\n[piracy]\n\t#Enable feature.\n\tenabled = true\n\t#Delay\n\tdelay = 0\n\n[greeting]\n\tenabled = true\n";

    #[test]
    fn only_the_piracy_section_is_switched_off() {
        let out = disable_piracy_notice(SAMPLE).unwrap();
        assert_eq!(out, SAMPLE.replacen("[piracy]\n\t#Enable feature.\n\tenabled = true", "[piracy]\n\t#Enable feature.\n\tenabled = false", 1));
        assert_eq!(out.matches("enabled = true").count(), 2, "general y greeting siguen igual");
    }

    #[test]
    fn nothing_to_do_when_already_off_or_absent() {
        assert!(disable_piracy_notice(&SAMPLE.replace("[piracy]\n\t#Enable feature.\n\tenabled = true", "[piracy]\n\tenabled = false")).is_none());
        assert!(disable_piracy_notice("[general]\n\tenabled = true\n").is_none());
        assert!(disable_piracy_notice("").is_none());
    }

    #[test]
    fn windows_line_endings_are_kept() {
        let crlf = SAMPLE.replace('\n', "\r\n");
        let out = disable_piracy_notice(&crlf).unwrap();
        assert!(out.contains("\tenabled = false\r\n"));
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
    }

    #[test]
    fn rewrites_the_file_in_place_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!silence_offline_notice(dir.path()).unwrap(), "sin el mod no hay nada que hacer");
        let path = dir.path().join(CONFIG);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, SAMPLE).unwrap();
        assert!(silence_offline_notice(dir.path()).unwrap());
        assert!(!silence_offline_notice(dir.path()).unwrap());
        assert!(std::fs::read_to_string(&path).unwrap().contains("[piracy]\n\t#Enable feature.\n\tenabled = false"));
        assert!(!path.with_extension("toml.part").exists());
    }
}
