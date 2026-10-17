use crate::LoaderError;
use url::Url;

/// Repositorios Maven desde los que se aceptan librerias de loaders (spec 11 §7: allowlist por proveedor).
const TRUSTED_HOSTS: &[&str] = &[
    "maven.fabricmc.net",
    "libraries.minecraft.net",
    "repo1.maven.org",
    "maven.quiltmc.org",
    "maven.neoforged.net",
    "maven.minecraftforge.net",
];

/// Solo HTTPS y solo hosts conocidos.
pub fn ensure_trusted(url: &str) -> Result<(), LoaderError> {
    let parsed = Url::parse(url).map_err(|_| LoaderError::UntrustedSource(url.to_string()))?;
    let trusted = parsed.scheme() == "https" && parsed.host_str().is_some_and(|h| TRUSTED_HOSTS.contains(&h));
    if trusted {
        Ok(())
    } else {
        Err(LoaderError::UntrustedSource(parsed.host_str().unwrap_or("?").to_string()))
    }
}

/// Versiones de Minecraft/loader llegan a URLs y rutas: solo caracteres de version.
pub fn validate_component(value: &str) -> Result<(), LoaderError> {
    let ok = !value.is_empty()
        && value.len() <= 64
        && value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        && !value.starts_with('.');
    if ok {
        Ok(())
    } else {
        Err(LoaderError::InvalidComponent(value.chars().take(32).collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_on_known_hosts() {
        assert!(ensure_trusted("https://maven.fabricmc.net/net/x.jar").is_ok());
        assert!(ensure_trusted("http://maven.fabricmc.net/x.jar").is_err());
        assert!(ensure_trusted("https://evil.example.com/x.jar").is_err());
        assert!(ensure_trusted("https://maven.fabricmc.net.evil.com/x.jar").is_err());
        assert!(ensure_trusted("not a url").is_err());
    }

    #[test]
    fn component_validation() {
        for ok in ["1.21.4", "0.16.9", "24w14a", "1.21-pre1", "1.20.1+build.2"] {
            assert!(validate_component(ok).is_ok(), "{ok}");
        }
        for bad in ["", "../x", "a/b", "a b", ".hidden", "a?b=c", &"x".repeat(65)] {
            assert!(validate_component(bad).is_err(), "{bad}");
        }
    }
}
