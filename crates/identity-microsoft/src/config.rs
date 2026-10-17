use url::Url;

pub const SCOPE: &str = "XboxLive.signin offline_access";

/// Endpoints de la cadena de autenticacion. Los valores por defecto son los reales; las pruebas
/// los apuntan a un servidor local.
#[derive(Debug, Clone)]
pub struct MicrosoftEndpoints {
    pub authorize: String,
    pub token: String,
    pub xbox_user: String,
    pub xsts: String,
    pub minecraft_login: String,
    pub minecraft_entitlements: String,
    pub minecraft_profile: String,
}

impl Default for MicrosoftEndpoints {
    fn default() -> Self {
        Self {
            authorize: "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize".into(),
            token: "https://login.microsoftonline.com/consumers/oauth2/v2.0/token".into(),
            xbox_user: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft_login: "https://api.minecraftservices.com/authentication/login_with_xbox".into(),
            minecraft_entitlements: "https://api.minecraftservices.com/entitlements/mcstore".into(),
            minecraft_profile: "https://api.minecraftservices.com/minecraft/profile".into(),
        }
    }
}

impl MicrosoftEndpoints {
    /// Todos los endpoints deben ser HTTPS (o http contra loopback, solo para pruebas locales).
    pub fn validate(&self) -> Result<(), ConfigError> {
        for url in [
            &self.authorize,
            &self.token,
            &self.xbox_user,
            &self.xsts,
            &self.minecraft_login,
            &self.minecraft_entitlements,
            &self.minecraft_profile,
        ] {
            ensure_secure_endpoint(url)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("el client id de Microsoft no tiene el formato esperado")]
    InvalidClientId,
    #[error("endpoint inseguro: {0}")]
    InsecureEndpoint(String),
}

#[derive(Debug, Clone, Default)]
pub struct MicrosoftConfig {
    /// Id de la app registrada (cliente publico). `None` = funcion deshabilitada.
    pub client_id: Option<String>,
    pub endpoints: MicrosoftEndpoints,
}

impl MicrosoftConfig {
    /// Config con el client id dado, validado. Cadena vacia = deshabilitado.
    pub fn with_client_id(client_id: Option<&str>) -> Result<Self, ConfigError> {
        let client_id = match client_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => Some(validate_client_id(id)?),
            None => None,
        };
        Ok(Self { client_id, endpoints: MicrosoftEndpoints::default() })
    }

    pub fn is_enabled(&self) -> bool {
        self.client_id.is_some()
    }
}

/// Los client id de Azure son GUID (`8-4-4-4-12` hexadecimales).
pub fn validate_client_id(id: &str) -> Result<String, ConfigError> {
    let parts: Vec<&str> = id.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    let ok = parts.len() == 5
        && parts.iter().zip(lens).all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()));
    if ok { Ok(id.to_lowercase()) } else { Err(ConfigError::InvalidClientId) }
}

pub fn ensure_secure_endpoint(url: &str) -> Result<(), ConfigError> {
    let parsed = Url::parse(url).map_err(|_| ConfigError::InsecureEndpoint(url.chars().take(40).collect()))?;
    let loopback = matches!(parsed.host_str(), Some("127.0.0.1") | Some("localhost") | Some("[::1]"));
    let ok = parsed.scheme() == "https" || (parsed.scheme() == "http" && loopback);
    if ok { Ok(()) } else { Err(ConfigError::InsecureEndpoint(parsed.host_str().unwrap_or("?").to_string())) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_id_must_be_a_guid() {
        assert!(validate_client_id("00000000-0000-0000-0000-000000000000").is_ok());
        assert_eq!(validate_client_id("ABCDEF12-1234-1234-1234-1234567890AB").unwrap(), "abcdef12-1234-1234-1234-1234567890ab");
        for bad in ["", "not-a-guid", "0000", "00000000-0000-0000-0000-00000000000g", "00000000000000000000000000000000"] {
            assert!(validate_client_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn disabled_without_client_id() {
        assert!(!MicrosoftConfig::with_client_id(None).unwrap().is_enabled());
        assert!(!MicrosoftConfig::with_client_id(Some("  ")).unwrap().is_enabled());
        assert!(MicrosoftConfig::with_client_id(Some("00000000-0000-0000-0000-000000000001")).unwrap().is_enabled());
        assert!(MicrosoftConfig::with_client_id(Some("garbage")).is_err());
    }

    #[test]
    fn default_endpoints_are_https_and_remote_http_is_rejected() {
        assert!(MicrosoftEndpoints::default().validate().is_ok());
        assert!(ensure_secure_endpoint("http://127.0.0.1:8080/x").is_ok());
        assert!(ensure_secure_endpoint("http://login.example.com/x").is_err());
        assert!(ensure_secure_endpoint("ftp://x").is_err());
    }
}
