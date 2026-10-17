use loaders::{
    overlay_from_profile, parse_loader_versions, FabricProvider, LoaderInstallContext, LoaderProvider, QuiltProvider,
};
use minecraft_meta::LoaderProfile;
use std::sync::Arc;

const PROFILE: &str = r#"{
  "id": "fabric-loader-0.16.9-1.21.4",
  "inheritsFrom": "1.21.4",
  "releaseTime": "2024-12-03T10:00:00+0000",
  "time": "2024-12-03T10:00:00+0000",
  "type": "release",
  "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
  "arguments": { "game": [], "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "] },
  "libraries": [
    { "name": "net.fabricmc:fabric-loader:0.16.9", "url": "https://maven.fabricmc.net/", "sha1": "aaaa", "size": 10 },
    { "name": "net.fabricmc:intermediary:1.21.4", "url": "https://maven.fabricmc.net/" }
  ]
}"#;

const VERSIONS: &str = r#"[
  { "loader": { "version": "0.16.9", "stable": true }, "intermediary": { "version": "1.21.4" } },
  { "loader": { "version": "0.16.10-beta", "stable": false }, "intermediary": { "version": "1.21.4" } }
]"#;

#[test]
fn parses_loader_versions() {
    let versions = parse_loader_versions(VERSIONS.as_bytes()).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, "0.16.9");
    assert!(versions[0].stable);
    assert!(!versions[1].stable);
}

#[test]
fn profile_becomes_overlay() {
    let profile: LoaderProfile = serde_json::from_str(PROFILE).unwrap();
    let overlay = overlay_from_profile(&profile);

    assert_eq!(overlay.main_class.as_deref(), Some("net.fabricmc.loader.impl.launch.knot.KnotClient"));
    assert_eq!(overlay.libraries.len(), 2);
    assert_eq!(overlay.arguments.unwrap().jvm.len(), 1);
}

#[test]
fn profile_id_follows_fabric_convention() {
    assert_eq!(FabricProvider::profile_id("1.21.4", "0.16.9"), "fabric-loader-0.16.9-1.21.4");
}

#[tokio::test]
async fn load_installed_requires_profile_and_all_libraries() {
    let dir = tempfile::tempdir().unwrap();
    let provider = FabricProvider::create(reqwest::Client::new());
    let load = || provider.load_installed(dir.path(), "1.21.4", "0.16.9");

    assert!(load().await.unwrap().is_none(), "sin perfil no hay instalacion");

    let id = "fabric-loader-0.16.9-1.21.4";
    let profile_dir = dir.path().join("versions").join(id);
    std::fs::create_dir_all(&profile_dir).unwrap();
    std::fs::write(profile_dir.join(format!("{id}.json")), PROFILE).unwrap();
    assert!(load().await.unwrap().is_none(), "faltan librerias");

    let libs = dir.path().join("libraries");
    for rel in [
        "net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar",
        "net/fabricmc/intermediary/1.21.4/intermediary-1.21.4.jar",
    ] {
        let p = libs.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"jar").unwrap();
    }
    let installed = load().await.unwrap().expect("instalacion completa");
    assert_eq!(installed.profile_id, id);
    assert!(installed.overlay.main_class.is_some());
}

#[tokio::test]
async fn rejects_path_like_versions() {
    let dir = tempfile::tempdir().unwrap();
    let provider = FabricProvider::create(reqwest::Client::new());
    assert!(provider.load_installed(dir.path(), "../..", "0.16.9").await.is_err());
    assert!(provider.versions("1.21/../x").await.is_err());
}

/// Contra los servidores reales de Fabric. `cargo test -p loaders -- --ignored`
#[tokio::test]
#[ignore = "requiere internet"]
async fn real_fabric_install() {
    let dir = tempfile::tempdir().unwrap();
    let provider = FabricProvider::create(reqwest::Client::new());

    let versions = provider.versions("1.20.1").await.unwrap();
    let latest_stable = versions.iter().find(|v| v.stable).expect("alguna version estable");

    let ctx = LoaderInstallContext {
        data_dir: dir.path(),
        minecraft_version: "1.20.1",
        loader_version: &latest_stable.version,
        concurrency: 4,
        progress: Arc::new(|_, _| {}),
    };
    let installed = provider.install(&ctx).await.unwrap();
    assert!(installed.overlay.main_class.unwrap().contains("Knot"));

    let again = provider.load_installed(dir.path(), "1.20.1", &latest_stable.version).await.unwrap();
    assert!(again.is_some(), "la instalacion debe poder cargarse sin red");
}

#[test]
fn quilt_versions_without_stable_flag_are_classified_by_name() {
    let json = br#"[
      { "loader": { "version": "0.27.1" } },
      { "loader": { "version": "0.28.0-beta.2" } }
    ]"#;
    let versions = parse_loader_versions(json).unwrap();
    assert!(versions[0].stable);
    assert!(!versions[1].stable);
}

#[test]
fn quilt_profile_ids_use_the_quilt_prefix() {
    assert_eq!(QuiltProvider::profile_id("1.21.4", "0.27.1"), "quilt-loader-0.27.1-1.21.4");
    let provider = QuiltProvider::create(reqwest::Client::new());
    assert_eq!(provider.profile_id("1.21.4", "0.27.1"), "quilt-loader-0.27.1-1.21.4");
    assert_eq!(provider.kind(), instance_manager::LoaderKind::Quilt);
}

/// Contra los servidores reales de Quilt. `cargo test -p loaders -- --ignored`
#[tokio::test]
#[ignore = "requiere internet"]
async fn real_quilt_install() {
    let dir = tempfile::tempdir().unwrap();
    let provider = QuiltProvider::create(reqwest::Client::new());

    let versions = provider.versions("1.20.1").await.unwrap();
    let stable = versions.iter().find(|v| v.stable).expect("alguna version estable");
    let ctx = LoaderInstallContext {
        data_dir: dir.path(),
        minecraft_version: "1.20.1",
        loader_version: &stable.version,
        concurrency: 4,
        progress: Arc::new(|_, _| {}),
    };
    let installed = provider.install(&ctx).await.unwrap();
    assert!(installed.overlay.main_class.unwrap().contains("Knot"));
    assert!(provider.load_installed(dir.path(), "1.20.1", &stable.version).await.unwrap().is_some());
}
