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
