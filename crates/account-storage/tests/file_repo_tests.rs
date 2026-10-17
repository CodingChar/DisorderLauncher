use account_storage::{AccountRepository, FileAccountRepository};
use chrono::Utc;
use identity_api::*;

fn make_account(id: &str, name: &str) -> AccountRecord {
    AccountRecord {
        id: id.into(),
        provider: IdentityProviderKind::Local,
        label: name.into(),
        created_at: Utc::now(),
        last_used_at: None,
        provider_data: ProviderAccountData::Local(LocalAccountData {
            display_name: name.into(),
            local_uuid: format!("uuid-{id}"),
            skin_id: None,
        }),
    }
}

#[tokio::test]
async fn crud_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let repo = FileAccountRepository::new(dir.path().join("accounts"));

    assert!(repo.list().await.unwrap().is_empty());

    let acc = make_account("a1", "Steve");
    repo.save(&acc).await.unwrap();

    let fetched = repo.get("a1").await.unwrap().unwrap();
    assert_eq!(fetched.label, "Steve");

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);

    repo.delete("a1").await.unwrap();
    assert!(repo.get("a1").await.unwrap().is_none());
    assert!(repo.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn default_account() {
    let dir = tempfile::tempdir().unwrap();
    let repo = FileAccountRepository::new(dir.path().join("accounts"));

    assert!(repo.get_default().await.unwrap().is_none());

    let acc = make_account("a1", "Steve");
    repo.save(&acc).await.unwrap();
    repo.set_default("a1").await.unwrap();
    assert_eq!(repo.get_default().await.unwrap().as_deref(), Some("a1"));

    repo.delete("a1").await.unwrap();
    assert!(repo.get_default().await.unwrap().is_none());
}

#[tokio::test]
async fn set_default_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let repo = FileAccountRepository::new(dir.path().join("accounts"));

    let result = repo.set_default("nope").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn save_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let repo = FileAccountRepository::new(dir.path().join("accounts"));

    let mut acc = make_account("a1", "Steve");
    repo.save(&acc).await.unwrap();

    acc.label = "Alex".into();
    repo.save(&acc).await.unwrap();

    let fetched = repo.get("a1").await.unwrap().unwrap();
    assert_eq!(fetched.label, "Alex");
    assert_eq!(repo.list().await.unwrap().len(), 1);
}

fn raw_account(dir: &std::path::Path, file: &str, json: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(file), json).unwrap();
}

const LEGACY_V0: &str = r#"{
  "id": "acc_local_legacy",
  "provider": "Local",
  "label": "Viejo",
  "created_at": "2025-01-01T00:00:00Z",
  "last_used_at": null,
  "provider_data": { "type": "local", "display_name": "Viejo", "local_uuid": "u-1", "skin_id": null }
}"#;

#[tokio::test]
async fn legacy_files_are_migrated_with_a_backup() {
    use account_storage::StorageNotice;
    let dir = tempfile::tempdir().unwrap();
    let accounts = dir.path().join("accounts");
    raw_account(&accounts, "acc_local_legacy.json", LEGACY_V0);
    let repo = FileAccountRepository::new(&accounts);

    let list = repo.list().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].label, "Viejo");
    assert_eq!(list[0].provider, IdentityProviderKind::Local);

    // El original se conserva y el archivo vigente ya tiene el formato nuevo.
    assert_eq!(std::fs::read_to_string(accounts.join("acc_local_legacy.json.bak-v0")).unwrap(), LEGACY_V0);
    let migrated: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(accounts.join("acc_local_legacy.json")).unwrap()).unwrap();
    assert_eq!(migrated["schemaVersion"], 1);
    assert_eq!(migrated["providerData"]["displayName"], "Viejo");
    assert_eq!(migrated["provider"], "local");
    assert_eq!(repo.take_notices(), vec![StorageNotice::Migrated { account: "acc_local_legacy".into(), from_version: 0 }]);

    // Una segunda lectura ya no migra nada.
    repo.list().await.unwrap();
    assert!(repo.take_notices().is_empty());
}

#[tokio::test]
async fn corrupt_and_future_files_do_not_hide_healthy_accounts() {
    use account_storage::StorageNotice;
    let dir = tempfile::tempdir().unwrap();
    let accounts = dir.path().join("accounts");
    let repo = FileAccountRepository::new(&accounts);
    repo.save(&make_account("acc_good", "Steve")).await.unwrap();
    raw_account(&accounts, "acc_broken.json", "{ not json");
    raw_account(&accounts, "acc_future.json", r#"{"schemaVersion": 99, "id": "acc_future"}"#);
    raw_account(&accounts, "acc_wrongshape.json", r#"{"schemaVersion": 1, "id": 5}"#);

    let list = repo.list().await.unwrap();
    assert_eq!(list.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["acc_good"]);

    let notices = repo.take_notices();
    assert!(notices.contains(&StorageNotice::Quarantined { file_name: "acc_broken.json".into() }));
    assert!(notices.contains(&StorageNotice::Quarantined { file_name: "acc_wrongshape.json".into() }));
    assert!(notices.contains(&StorageNotice::NewerFormat { file_name: "acc_future.json".into() }));
    assert!(accounts.join("acc_broken.json.corrupt").exists());
    assert!(accounts.join("acc_future.json").exists(), "los archivos de versiones futuras no se tocan");
}

#[tokio::test]
async fn account_ids_cannot_escape_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let accounts = dir.path().join("accounts");
    let repo = FileAccountRepository::new(&accounts);
    repo.save(&make_account("acc_a", "A")).await.unwrap();
    let victim = dir.path().join("secret.json");
    std::fs::write(&victim, "{}").unwrap();
    std::fs::write(accounts.join("_default.txt"), "../secret").unwrap();

    for id in ["../secret", r"..\secret", "acc_../../secret", "", "acc_a/../../secret", "a.b"] {
        assert!(repo.get(id).await.unwrap().is_none(), "{id}");
        assert!(repo.delete(id).await.is_err(), "{id}");
        assert!(repo.set_default(id).await.is_err(), "{id}");
    }
    assert!(victim.exists(), "ningun archivo fuera de accounts/ pudo tocarse");
    assert_eq!(repo.get_default().await.unwrap(), None, "un predeterminado manipulado se ignora");
    let mut wrapper = make_account("../evil", "X");
    wrapper.id = "../evil".into();
    assert!(repo.save(&wrapper).await.is_err());
}
