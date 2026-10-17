use credential_store::{CredentialStore, InMemoryCredentialStore};

#[tokio::test]
async fn put_get_delete() {
    let store = InMemoryCredentialStore::new();

    assert!(store.get_secret("key1").await.unwrap().is_none());

    store.put_secret("key1", "s3cret").await.unwrap();
    assert_eq!(
        store.get_secret("key1").await.unwrap().as_deref(),
        Some("s3cret")
    );

    store.put_secret("key1", "updated").await.unwrap();
    assert_eq!(
        store.get_secret("key1").await.unwrap().as_deref(),
        Some("updated")
    );

    store.delete_secret("key1").await.unwrap();
    assert!(store.get_secret("key1").await.unwrap().is_none());
}

#[tokio::test]
async fn delete_nonexistent_is_ok() {
    let store = InMemoryCredentialStore::new();
    store.delete_secret("nope").await.unwrap();
}
