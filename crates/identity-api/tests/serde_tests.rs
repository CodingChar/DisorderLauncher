use chrono::Utc;
use identity_api::*;

fn sample_local_account() -> AccountRecord {
    AccountRecord {
        id: "acc-001".into(),
        provider: IdentityProviderKind::Local,
        label: "Steve".into(),
        created_at: Utc::now(),
        last_used_at: None,
        provider_data: ProviderAccountData::Local(LocalAccountData {
            display_name: "Steve".into(),
            local_uuid: "offline:steve-uuid".into(),
            skin_id: None,
        }),
    }
}

fn sample_microsoft_account() -> AccountRecord {
    AccountRecord {
        id: "acc-002".into(),
        provider: IdentityProviderKind::Microsoft,
        label: "GamerTag".into(),
        created_at: Utc::now(),
        last_used_at: Some(Utc::now()),
        provider_data: ProviderAccountData::Microsoft(MicrosoftAccountMetadata {
            minecraft_uuid: Some("ab12cd34".into()),
            minecraft_name: Some("GamerTag".into()),
            secure_refresh_token_key: Some("keyring://ms-refresh-acc-002".into()),
            last_successful_auth: Some(Utc::now()),
        }),
    }
}

#[test]
fn local_account_roundtrip() {
    let record = sample_local_account();
    let json = serde_json::to_string_pretty(&record).unwrap();
    let parsed: AccountRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, record.id);
    assert_eq!(parsed.provider, IdentityProviderKind::Local);
    match &parsed.provider_data {
        ProviderAccountData::Local(data) => {
            assert_eq!(data.display_name, "Steve");
        }
        _ => panic!("expected Local provider data"),
    }
}

#[test]
fn microsoft_account_roundtrip() {
    let record = sample_microsoft_account();
    let json = serde_json::to_string_pretty(&record).unwrap();
    let parsed: AccountRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.id, "acc-002");
    assert_eq!(parsed.provider, IdentityProviderKind::Microsoft);
    match &parsed.provider_data {
        ProviderAccountData::Microsoft(meta) => {
            assert_eq!(meta.minecraft_name.as_deref(), Some("GamerTag"));
        }
        _ => panic!("expected Microsoft provider data"),
    }
}

#[test]
fn provider_data_tagged_serde() {
    let local = ProviderAccountData::Local(LocalAccountData {
        display_name: "Alex".into(),
        local_uuid: "uuid".into(),
        skin_id: None,
    });
    let json = serde_json::to_string(&local).unwrap();
    assert!(json.contains(r#""type":"local""#));

    let ms = ProviderAccountData::Microsoft(MicrosoftAccountMetadata {
        minecraft_uuid: None,
        minecraft_name: None,
        secure_refresh_token_key: None,
        last_successful_auth: None,
    });
    let json = serde_json::to_string(&ms).unwrap();
    assert!(json.contains(r#""type":"microsoft""#));
}

#[test]
fn auth_state_serde() {
    let states = vec![
        AuthState::LocalReady,
        AuthState::MicrosoftDisconnected,
        AuthState::Error(AuthErrorKind::NetworkUnavailable),
    ];
    for state in &states {
        let json = serde_json::to_string(state).unwrap();
        let parsed: AuthState = serde_json::from_str(&json).unwrap();
        let re_json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, re_json);
    }
}

#[test]
fn resolved_identity_serde() {
    let identity = ResolvedIdentity {
        profile_id: "p-1".into(),
        provider: IdentityProviderKind::Local,
        display_name: "Steve".into(),
        game_uuid: "some-uuid".into(),
        session: GameSessionMaterial {
            access_token: "0".into(),
            token_type: "legacy".into(),
        },
        skin: None,
    };
    let json = serde_json::to_string(&identity).unwrap();
    let parsed: ResolvedIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.profile_id, "p-1");
    assert_eq!(parsed.session.access_token, "0");
}

#[test]
fn identity_provider_kind_display() {
    assert_eq!(IdentityProviderKind::Local.to_string(), "local");
    assert_eq!(IdentityProviderKind::Microsoft.to_string(), "microsoft");
}
