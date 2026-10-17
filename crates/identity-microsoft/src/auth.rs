use crate::{ConfigError, MicrosoftConfig, Secret, SCOPE};
use serde::Deserialize;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// Nombres de derecho (`entitlements`) que indican que la cuenta puede jugar Minecraft Java.
const OWNERSHIP_ITEMS: &[&str] = &["product_minecraft", "game_minecraft", "product_game_pass_pc", "product_game_pass_ultimate"];

#[derive(Debug)]
pub enum AuthStepError {
    Network(reqwest::Error),
    /// `invalid_grant`: el refresh token fue revocado o expiro; hay que volver a iniciar sesion.
    InvalidGrant,
    TokenExchange(String),
    Xbox(String),
    /// Mensaje legible para el usuario (ver `xsts_message`).
    Xsts(String),
    /// Minecraft Services rechazo la aplicacion o el token.
    ServicesRejected,
    NoOwnership,
    NoProfile,
    Malformed(&'static str),
}

impl From<reqwest::Error> for AuthStepError {
    fn from(e: reqwest::Error) -> Self {
        // El detalle de reqwest puede incluir la URL; nunca incluye cuerpos ni tokens.
        Self::Network(e)
    }
}

#[derive(Debug)]
pub struct MsTokens {
    pub access_token: Secret,
    pub refresh_token: Option<Secret>,
    pub expires_in: u64,
}

#[derive(Debug)]
pub struct XboxToken {
    pub token: Secret,
    pub user_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McProfile {
    /// UUID sin guiones, como lo devuelve Minecraft Services.
    pub id: String,
    pub name: String,
    pub skin_id: Option<String>,
}

#[derive(Debug)]
pub struct McSession {
    pub access_token: Secret,
    pub expires_in: u64,
    pub profile: McProfile,
}

pub struct MicrosoftAuthClient {
    http: reqwest::Client,
    config: MicrosoftConfig,
    client_id: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
}

#[derive(Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}

#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<Xui>,
}

#[derive(Deserialize)]
struct Xui {
    uhs: String,
}

#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr")]
    code: Option<u64>,
}

#[derive(Deserialize)]
struct McLoginResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct Entitlements {
    #[serde(default)]
    items: Vec<EntitlementItem>,
}

#[derive(Deserialize)]
struct EntitlementItem {
    name: String,
}

#[derive(Deserialize)]
struct ProfileResponse {
    id: String,
    name: String,
    #[serde(default)]
    skins: Vec<SkinResponse>,
}

#[derive(Deserialize)]
struct SkinResponse {
    id: String,
    #[serde(default)]
    state: String,
}

/// Mensajes comprensibles para los codigos `XErr` mas frecuentes de XSTS.
pub fn xsts_message(code: Option<u64>) -> String {
    match code {
        Some(2148916233) => "Tu cuenta de Microsoft no tiene un perfil de Xbox. Crea uno en xbox.com y vuelve a intentarlo.".into(),
        Some(2148916235) => "Xbox no esta disponible en tu pais o region.".into(),
        Some(2148916236) | Some(2148916237) => "Tu cuenta necesita verificacion de edad adulta en Xbox.".into(),
        Some(2148916238) => "Tu cuenta pertenece a un menor: debe agregarse a una familia de Microsoft para jugar.".into(),
        Some(other) => format!("Xbox rechazo la solicitud (codigo {other})."),
        None => "Xbox rechazo la solicitud.".into(),
    }
}

impl MicrosoftAuthClient {
    /// `None` si la configuracion no tiene client id (funcion deshabilitada).
    pub fn new(http: reqwest::Client, config: MicrosoftConfig) -> Result<Option<Self>, ConfigError> {
        config.endpoints.validate()?;
        let Some(client_id) = config.client_id.clone() else { return Ok(None) };
        Ok(Some(Self { http, config, client_id }))
    }

    /// URL que se abre en el navegador del sistema.
    pub fn authorize_url(&self, redirect_uri: &str, state: &str, code_challenge: &str) -> String {
        let mut url = url::Url::parse(&self.config.endpoints.authorize).expect("endpoint validado");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", SCOPE)
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("prompt", "select_account");
        url.into()
    }

    pub async fn exchange_code(&self, code: &str, redirect_uri: &str, verifier: &Secret) -> Result<MsTokens, AuthStepError> {
        self.token_request(&[
            ("client_id", self.client_id.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier.expose()),
            ("scope", SCOPE),
        ])
        .await
    }

    pub async fn refresh(&self, refresh_token: &Secret) -> Result<MsTokens, AuthStepError> {
        self.token_request(&[
            ("client_id", self.client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.expose()),
            ("scope", SCOPE),
        ])
        .await
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> Result<MsTokens, AuthStepError> {
        let response = self
            .http
            .post(&self.config.endpoints.token)
            .timeout(REQUEST_TIMEOUT)
            .form(form)
            .send()
            .await?;
        let status = response.status();
        let body = response.bytes().await?;

        if !status.is_success() {
            let code = serde_json::from_slice::<OAuthError>(&body).map(|e| e.error).unwrap_or_default();
            return Err(if code == "invalid_grant" {
                AuthStepError::InvalidGrant
            } else if status.is_server_error() {
                // Error del servidor: transitorio, no invalida nada.
                AuthStepError::TokenExchange(format!("http {}", status.as_u16()))
            } else {
                AuthStepError::TokenExchange(if code.is_empty() { format!("http {}", status.as_u16()) } else { code })
            });
        }
        let parsed: TokenResponse = serde_json::from_slice(&body).map_err(|_| AuthStepError::Malformed("token"))?;
        Ok(MsTokens {
            access_token: Secret::new(parsed.access_token),
            refresh_token: parsed.refresh_token.filter(|t| !t.is_empty()).map(Secret::new),
            expires_in: parsed.expires_in,
        })
    }

    pub async fn xbox_authenticate(&self, ms_access: &Secret) -> Result<XboxToken, AuthStepError> {
        let body = serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms_access.expose()),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        });
        let response = self.http.post(&self.config.endpoints.xbox_user).timeout(REQUEST_TIMEOUT).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(AuthStepError::Xbox(format!("http {}", response.status().as_u16())));
        }
        parse_xbox(response).await
    }

    pub async fn xsts_authorize(&self, xbl: &XboxToken) -> Result<XboxToken, AuthStepError> {
        let body = serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token.expose()] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        });
        let response = self.http.post(&self.config.endpoints.xsts).timeout(REQUEST_TIMEOUT).json(&body).send().await?;
        let status = response.status();
        if !status.is_success() {
            let code = response.json::<XstsError>().await.ok().and_then(|e| e.code);
            return Err(AuthStepError::Xsts(xsts_message(code)));
        }
        parse_xbox(response).await
    }

    pub async fn minecraft_login(&self, xsts: &XboxToken) -> Result<(Secret, u64), AuthStepError> {
        let identity = format!("XBL3.0 x={};{}", xsts.user_hash, xsts.token.expose());
        let response = self
            .http
            .post(&self.config.endpoints.minecraft_login)
            .timeout(REQUEST_TIMEOUT)
            .json(&serde_json::json!({ "identityToken": identity }))
            .send()
            .await?;
        let status = response.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(AuthStepError::ServicesRejected);
        }
        if !status.is_success() {
            return Err(AuthStepError::TokenExchange(format!("minecraft http {}", status.as_u16())));
        }
        let parsed: McLoginResponse = response.json().await.map_err(|_| AuthStepError::Malformed("minecraft login"))?;
        Ok((Secret::new(parsed.access_token), parsed.expires_in))
    }

    pub async fn owns_minecraft(&self, mc_token: &Secret) -> Result<bool, AuthStepError> {
        let response = self
            .http
            .get(&self.config.endpoints.minecraft_entitlements)
            .timeout(REQUEST_TIMEOUT)
            .bearer_auth(mc_token.expose())
            .send()
            .await?;
        if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
            return Err(AuthStepError::ServicesRejected);
        }
        if !response.status().is_success() {
            return Err(AuthStepError::TokenExchange(format!("entitlements http {}", response.status().as_u16())));
        }
        let parsed: Entitlements = response.json().await.map_err(|_| AuthStepError::Malformed("entitlements"))?;
        Ok(parsed.items.iter().any(|i| OWNERSHIP_ITEMS.contains(&i.name.as_str())))
    }

    pub async fn profile(&self, mc_token: &Secret) -> Result<McProfile, AuthStepError> {
        let response = self
            .http
            .get(&self.config.endpoints.minecraft_profile)
            .timeout(REQUEST_TIMEOUT)
            .bearer_auth(mc_token.expose())
            .send()
            .await?;
        match response.status().as_u16() {
            404 => return Err(AuthStepError::NoProfile),
            401 | 403 => return Err(AuthStepError::ServicesRejected),
            s if !(200..300).contains(&s) => return Err(AuthStepError::TokenExchange(format!("profile http {s}"))),
            _ => {}
        }
        let parsed: ProfileResponse = response.json().await.map_err(|_| AuthStepError::Malformed("profile"))?;
        let skin_id = parsed.skins.iter().find(|s| s.state.eq_ignore_ascii_case("ACTIVE")).map(|s| s.id.clone());
        Ok(McProfile { id: parsed.id, name: parsed.name, skin_id })
    }

    /// Cadena completa: token de Microsoft -> Xbox -> XSTS -> Minecraft -> propiedad -> perfil.
    pub async fn minecraft_session(&self, ms_access: &Secret) -> Result<McSession, AuthStepError> {
        let xbl = self.xbox_authenticate(ms_access).await?;
        let xsts = self.xsts_authorize(&xbl).await?;
        let (token, expires_in) = self.minecraft_login(&xsts).await?;
        if !self.owns_minecraft(&token).await? {
            return Err(AuthStepError::NoOwnership);
        }
        let profile = self.profile(&token).await?;
        Ok(McSession { access_token: token, expires_in, profile })
    }
}

async fn parse_xbox(response: reqwest::Response) -> Result<XboxToken, AuthStepError> {
    let parsed: XboxResponse = response.json().await.map_err(|_| AuthStepError::Malformed("xbox"))?;
    let user_hash = parsed.display_claims.xui.into_iter().next().map(|x| x.uhs).ok_or(AuthStepError::Malformed("xbox uhs"))?;
    Ok(XboxToken { token: Secret::new(parsed.token), user_hash })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xsts_messages_are_human() {
        assert!(xsts_message(Some(2148916233)).contains("perfil de Xbox"));
        assert!(xsts_message(Some(2148916238)).contains("familia"));
        assert!(xsts_message(Some(1)).contains("codigo 1"));
        assert!(!xsts_message(None).is_empty());
    }

    #[test]
    fn disabled_without_client_id_and_url_carries_pkce_and_state() {
        let http = reqwest::Client::new();
        assert!(MicrosoftAuthClient::new(http.clone(), MicrosoftConfig::default()).unwrap().is_none());

        let config = MicrosoftConfig::with_client_id(Some("00000000-0000-0000-0000-000000000001")).unwrap();
        let client = MicrosoftAuthClient::new(http, config).unwrap().unwrap();
        let url = url::Url::parse(&client.authorize_url("http://localhost:1234/callback", "st4te", "chal")).unwrap();
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(url.host_str(), Some("login.microsoftonline.com"));
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["state"], "st4te");
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["redirect_uri"], "http://localhost:1234/callback");
        assert!(q["scope"].contains("XboxLive.signin") && q["scope"].contains("offline_access"));
        assert!(!url.as_str().contains("client_secret"));
    }
}
