use std::fmt;
use zeroize::Zeroizing;

/// Cadena secreta (tokens, verifier): no se imprime en `Debug`/`Display` y se borra de memoria al soltarse.
#[derive(Clone)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(Zeroizing::new(value.into()))
    }

    /// Acceso explicito al valor; usar solo al enviarlo a su destino.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_leaks() {
        let s = Secret::new("super-secret-token");
        assert!(!format!("{s:?}").contains("super-secret"));
        assert_eq!(s.expose(), "super-secret-token");
    }
}
