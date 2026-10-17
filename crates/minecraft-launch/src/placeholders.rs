use crate::LaunchError;
use std::collections::HashMap;

/// Tabla central de sustitucion de variables `${...}` de los argumentos de
/// lanzamiento. Es el unico lugar donde se inyectan datos de identidad.
#[derive(Default)]
pub struct Placeholders {
    values: HashMap<&'static str, String>,
}

impl Placeholders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: &'static str, value: impl Into<String>) {
        self.values.insert(key, value.into());
    }

    /// Sustituye todos los `${clave}` de `template`. Una clave desconocida es un
    /// error: lanzar con un argumento literal `${...}` rompe el juego.
    pub fn apply(&self, template: &str) -> Result<String, LaunchError> {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;

        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find('}') else {
                out.push_str(&rest[start..]);
                return Ok(out);
            };
            let key = &after[..end];
            match self.values.get(key) {
                Some(value) => out.push_str(value),
                None => return Err(LaunchError::UnresolvedPlaceholder(key.to_string())),
            }
            rest = &after[end + 1..];
        }

        out.push_str(rest);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Placeholders {
        let mut p = Placeholders::new();
        p.set("auth_player_name", "Steve");
        p.set("version_name", "1.21");
        p
    }

    #[test]
    fn replaces_multiple_placeholders() {
        let out = table().apply("--username ${auth_player_name} --version ${version_name}").unwrap();
        assert_eq!(out, "--username Steve --version 1.21");
    }

    #[test]
    fn keeps_text_without_placeholders() {
        assert_eq!(table().apply("-Xmx2G").unwrap(), "-Xmx2G");
    }

    #[test]
    fn unknown_placeholder_is_error() {
        let err = table().apply("${nope}").unwrap_err();
        assert!(matches!(err, LaunchError::UnresolvedPlaceholder(k) if k == "nope"));
    }

    #[test]
    fn value_containing_placeholder_syntax_is_not_reexpanded() {
        let mut p = Placeholders::new();
        p.set("auth_player_name", "${version_name}");
        p.set("version_name", "1.21");
        assert_eq!(p.apply("${auth_player_name}").unwrap(), "${version_name}");
    }
}
