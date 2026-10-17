use crate::uuid_offline::offline_uuid;
use chrono::Utc;
use identity_api::{
    AccountRecord, IdentityError, IdentityProviderKind, LocalAccountData, ProviderAccountData,
};

pub const MIN_NAME_LEN: usize = 3;
pub const MAX_NAME_LEN: usize = 16;

/// Politica de nombres de Offline Gaming: sin espacios laterales, 3-16 caracteres,
/// solo letras ASCII, numeros y guion bajo (lo que acepta el cliente de Minecraft).
pub fn validate_local_name(name: &str) -> Result<String, IdentityError> {
    let name = name.trim();
    let len = name.chars().count();
    if !(MIN_NAME_LEN..=MAX_NAME_LEN).contains(&len) {
        return Err(IdentityError::LocalProfileInvalid(format!(
            "El nombre debe tener entre {MIN_NAME_LEN} y {MAX_NAME_LEN} caracteres."
        )));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(IdentityError::LocalProfileInvalid(
            "El nombre solo puede tener letras, numeros y guion bajo (_), sin espacios.".into(),
        ));
    }
    Ok(name.to_string())
}

/// Crea un perfil local nuevo. El `id` es independiente del nombre para que un
/// futuro renombrado no rompa las instancias que lo referencian.
pub fn new_local_account(name: &str) -> Result<AccountRecord, IdentityError> {
    let name = validate_local_name(name)?;
    Ok(AccountRecord {
        id: format!("acc_local_{}", uuid::Uuid::new_v4().simple()),
        provider: IdentityProviderKind::Local,
        label: name.clone(),
        created_at: Utc::now(),
        last_used_at: None,
        provider_data: ProviderAccountData::Local(LocalAccountData {
            local_uuid: offline_uuid(&name),
            display_name: name,
            skin_id: None,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names_and_trims() {
        assert_eq!(validate_local_name("  David_01 ").unwrap(), "David_01");
    }

    #[test]
    fn rejects_bad_names() {
        for bad in ["", "ab", "a very long name indeed", "con espacio", "\u{f1}and\u{fa}", "x-y", "a
b"] {
            assert!(validate_local_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn account_id_is_independent_of_name_and_uuid_is_offline_compatible() {
        let a = new_local_account("Steve").unwrap();
        let b = new_local_account("Steve").unwrap();
        assert_ne!(a.id, b.id);
        assert!(a.id.starts_with("acc_local_"));
        match a.provider_data {
            ProviderAccountData::Local(d) => assert_eq!(d.local_uuid, offline_uuid("Steve")),
            _ => panic!("debe ser local"),
        }
    }
}
