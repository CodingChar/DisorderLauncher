use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdentityProviderKind {
    Local,
    Microsoft,
}

impl fmt::Display for IdentityProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Microsoft => write!(f, "microsoft"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedIdentity {
    pub profile_id: String,
    pub provider: IdentityProviderKind,
    pub display_name: String,
    pub game_uuid: String,
    pub session: GameSessionMaterial,
    pub skin: Option<SkinReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSessionMaterial {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinReference {
    pub id: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthState {
    LocalReady,
    MicrosoftDisconnected,
    MicrosoftAuthorizing,
    MicrosoftAuthenticated,
    MicrosoftRefreshRequired,
    MicrosoftOfflineCachedIdentity,
    MicrosoftExpired,
    Error(AuthErrorKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthErrorKind {
    ProviderNotConfigured,
    NetworkUnavailable,
    ProfileInvalid(String),
    TokenExchangeFailed,
    SecureStoreUnavailable,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    pub provider: IdentityProviderKind,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub provider_data: ProviderAccountData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderAccountData {
    #[serde(rename = "local")]
    Local(LocalAccountData),
    #[serde(rename = "microsoft")]
    Microsoft(MicrosoftAccountMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAccountData {
    pub display_name: String,
    pub local_uuid: String,
    pub skin_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccountMetadata {
    pub minecraft_uuid: Option<String>,
    pub minecraft_name: Option<String>,
    pub secure_refresh_token_key: Option<String>,
    pub last_successful_auth: Option<DateTime<Utc>>,
}
