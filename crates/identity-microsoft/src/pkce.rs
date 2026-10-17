use crate::Secret;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// Bytes aleatorios del sistema operativo (CSPRNG).
fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::fill(&mut buf).expect("el generador aleatorio del sistema no esta disponible");
    buf
}

/// Code verifier de PKCE: 64 bytes aleatorios -> 86 caracteres base64url (RFC 7636 pide 43-128).
pub fn generate_verifier() -> Secret {
    Secret::new(URL_SAFE_NO_PAD.encode(random_bytes::<64>()))
}

/// `code_challenge` con metodo S256.
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Valor `state` aleatorio contra CSRF/spoofing del callback.
pub fn generate_state() -> String {
    URL_SAFE_NO_PAD.encode(random_bytes::<32>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc7636_appendix_b_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(challenge_for(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn verifier_is_long_urlsafe_and_unique() {
        let a = generate_verifier();
        let b = generate_verifier();
        assert!((43..=128).contains(&a.expose().len()));
        assert!(a.expose().chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        assert_ne!(a.expose(), b.expose());
    }

    #[test]
    fn state_is_random_and_urlsafe() {
        let (a, b) = (generate_state(), generate_state());
        assert_ne!(a, b);
        assert!(a.len() >= 32 && a.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
