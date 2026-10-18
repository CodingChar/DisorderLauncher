use mods::{load_lock, ModManager, ModrinthClient, ModsService};
use test_support::FakeServer;

const VERSION: &str = r#"{
  "id": "ver1", "project_id": "proj1", "version_number": "1.2.3", "name": "Version bonita",
  "date_published": "2024-05-01T10:00:00.000000Z", "version_type": "release",
  "files": [], "dependencies": []
}"#;

#[tokio::test]
async fn files_known_to_modrinth_are_registered_and_the_rest_stay_local() {
    let server = FakeServer::start().await;
    server.route("POST /version_files", |request| {
        assert!(request.body.contains("\"algorithm\":\"sha1\""));
        assert!(request.body.contains("aaaa"));
        (200, format!(r#"{{"aaaa": {VERSION}}}"#))
    });
    server.respond("GET /projects", 200, r#"[{"id":"proj1","title":"Sodium"}]"#);

    let game = tempfile::tempdir().unwrap();
    let manager = ModManager::new(game.path());
    std::fs::create_dir_all(manager.mods_dir()).unwrap();
    std::fs::write(manager.mods_dir().join("sodium.jar"), b"x").unwrap();
    std::fs::write(manager.mods_dir().join("mio.jar"), b"y").unwrap();

    let client = ModrinthClient::new(reqwest::Client::new()).with_base_url(server.base.clone());
    let service = ModsService::new(reqwest::Client::new()).with_client(client);
    let files = vec![
        ("sodium.jar".to_string(), "AAAA".to_string()),
        ("mio.jar".to_string(), "bbbb".to_string()),
        ("ausente.jar".to_string(), "cccc".to_string()),
        ("../fuera.jar".to_string(), "dddd".to_string()),
    ];
    assert_eq!(service.register_known(&manager, &files).await.unwrap(), 1);

    let lock = load_lock(&manager.lock_file()).await;
    assert_eq!(lock.mods.len(), 1);
    let entry = &lock.mods[0];
    assert_eq!((entry.file.as_str(), entry.project_id.as_str(), entry.title.as_str()), ("sodium.jar", "proj1", "Sodium"));
    assert_eq!(entry.version_id, "ver1");
    assert_eq!(entry.sha1, "aaaa");
}
