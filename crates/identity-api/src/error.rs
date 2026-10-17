#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("local profile invalid: {0}")]
    LocalProfileInvalid(String),
    #[error("microsoft not configured")]
    MicrosoftNotConfigured,
    #[error("microsoft authorization cancelled")]
    MicrosoftAuthorizationCancelled,
    #[error("microsoft login timed out")]
    MicrosoftLoginTimedOut,
    #[error("microsoft state mismatch")]
    MicrosoftStateMismatch,
    #[error("microsoft session expired, sign in again")]
    MicrosoftReauthRequired,
    #[error("xbox authentication failed: {0}")]
    MicrosoftXboxAuthFailed(String),
    #[error("xsts authorization failed: {0}")]
    MicrosoftXstsFailed(String),
    #[error("minecraft services rejected this application")]
    MinecraftServicesRejectedApp,
    #[error("this account does not own minecraft java edition")]
    MinecraftOwnershipUnavailable,
    #[error("microsoft token exchange failed: {0}")]
    MicrosoftTokenExchangeFailed(String),
    #[error("secure store unavailable: {0}")]
    SecureStoreUnavailable(String),
    #[error("network unavailable")]
    NetworkUnavailable,
    #[error("{0}")]
    Other(String),
}
