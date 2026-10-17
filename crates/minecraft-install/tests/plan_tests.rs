use minecraft_install::*;
use minecraft_meta::{AssetIndex, AssetObject, VersionDescriptor};
use std::collections::HashMap;

const VERSION_JSON: &str = include_str!("../../minecraft-meta/tests/fixtures/version_1_21_4.json");

#[test]
fn build_plan_from_descriptor() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    assert_eq!(plan.version, "1.21.4");
    assert!(plan.client.url.contains("client.jar"));
    assert!(plan.client.path.ends_with("1.21.4.jar"));
    assert_eq!(plan.client.size, Some(26000000));
}

#[test]
fn plan_includes_asset_index() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    assert!(plan.asset_index.url.contains("18.json"));
    assert!(plan.asset_index.path.ends_with("18.json"));
}

#[test]
fn plan_java_version() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    assert_eq!(plan.java_major_version, Some(21));
}

#[test]
fn plan_filters_libraries_by_os() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    // The first library has no rules (always included)
    // The second has rules for windows only
    // On Windows: both libraries + 1 native; on other OS: only 1 library
    assert!(!plan.libraries.is_empty());

    let logging_lib = plan
        .libraries
        .iter()
        .find(|a| a.url.contains("logging"));
    assert!(logging_lib.is_some());
}

#[cfg(target_os = "windows")]
#[test]
fn plan_includes_natives_on_windows() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    assert!(!plan.natives.is_empty());
    assert!(plan.natives[0].url.contains("natives-windows"));
}

#[test]
fn build_asset_artifacts_from_index() {
    let mut objects = HashMap::new();
    objects.insert(
        "minecraft/sounds/click.ogg".to_string(),
        AssetObject {
            hash: "abc123def456789012345678901234567890abcd".to_string(),
            size: 1234,
        },
    );
    objects.insert(
        "minecraft/textures/block/stone.png".to_string(),
        AssetObject {
            hash: "ff0011223344556677889900aabbccddeeff0011".to_string(),
            size: 5678,
        },
    );

    let index = AssetIndex { objects, is_virtual: false, map_to_resources: false };
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let artifacts = build_asset_artifacts(&index, &game_dir);

    assert_eq!(artifacts.len(), 2);

    for artifact in &artifacts {
        assert!(artifact.url.starts_with("https://resources.download.minecraft.net/"));
        assert!(artifact.sha1.is_some());
        let hash = artifact.sha1.as_ref().unwrap();
        let prefix = &hash[..2];
        assert!(artifact.path.to_str().unwrap().contains(prefix));
    }
}

#[test]
fn empty_assets_before_index_download() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let game_dir = std::path::PathBuf::from("/tmp/minecraft");
    let plan = build_install_plan(&desc, &game_dir);

    assert!(plan.assets.is_empty());
}
