#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("local profile invalid: {0}")]
    LocalProfileInvalid(String),
    #[error("microsoft not configured")]
    MicrosoftNotConfigured,
    #[error("microsoft authorization cancelled")]
    MicrosoftAuthorizationCancelled,
    #[error("microsoft token exchange failed: {0}")]
    MicrosoftTokenExchangeFailed(String),
    #[error("secure store unavailable: {0}")]
    SecureStoreUnavailable(String),
    #[error("network unavailable")]
    NetworkUnavailable,
    #[error("{0}")]
    Other(String),
}
