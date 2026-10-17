use minecraft_meta::*;

const MANIFEST_JSON: &str = include_str!("fixtures/manifest_sample.json");
const VERSION_JSON: &str = include_str!("fixtures/version_1_21_4.json");

#[test]
fn parse_manifest() {
    let manifest: VersionManifest = serde_json::from_str(MANIFEST_JSON).unwrap();
    assert_eq!(manifest.latest.release, "1.21.4");
    assert_eq!(manifest.latest.snapshot, "25w02a");
    assert_eq!(manifest.versions.len(), 3);
}

#[test]
fn manifest_version_types() {
    let manifest: VersionManifest = serde_json::from_str(MANIFEST_JSON).unwrap();
    assert_eq!(manifest.versions[0].version_type, VersionType::Release);
    assert_eq!(manifest.versions[1].version_type, VersionType::Snapshot);
    assert_eq!(manifest.versions[2].version_type, VersionType::OldBeta);
}

#[test]
fn parse_version_descriptor() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    assert_eq!(desc.id, "1.21.4");
    assert_eq!(desc.main_class, "net.minecraft.client.main.Main");
    assert_eq!(desc.downloads.client.size, 26000000);
    assert_eq!(desc.assets, "18");
}

#[test]
fn version_descriptor_libraries() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    assert_eq!(desc.libraries.len(), 2);

    let lwjgl = &desc.libraries[1];
    assert_eq!(lwjgl.name, "org.lwjgl:lwjgl:3.3.3");
    assert!(lwjgl.natives.is_some());
    assert!(lwjgl.rules.is_some());

    let classifiers = lwjgl.downloads.as_ref().unwrap().classifiers.as_ref().unwrap();
    assert!(classifiers.contains_key("natives-windows"));
}

#[test]
fn version_descriptor_arguments() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let args = desc.arguments.as_ref().unwrap();

    let simple_count = args
        .game
        .iter()
        .filter(|a| matches!(a, ArgumentValue::Simple(_)))
        .count();
    assert!(simple_count >= 4);

    let conditional_count = args
        .game
        .iter()
        .filter(|a| matches!(a, ArgumentValue::Conditional { .. }))
        .count();
    assert_eq!(conditional_count, 1);
}

#[test]
fn version_descriptor_java_version() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    let java = desc.java_version.as_ref().unwrap();
    assert_eq!(java.component, "java-runtime-delta");
    assert_eq!(java.major_version, 21);
}

#[test]
fn version_descriptor_asset_index() {
    let desc: VersionDescriptor = serde_json::from_str(VERSION_JSON).unwrap();
    assert_eq!(desc.asset_index.id, "18");
    assert_eq!(desc.asset_index.total_size, 600000000);
    assert!(desc.asset_index.url.starts_with("https://"));
}

#[tokio::test]
async fn cache_manifest_roundtrip() {
    let manifest: VersionManifest = serde_json::from_str(MANIFEST_JSON).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache").join("manifest.json");

    save_manifest_cache(&manifest, &path).await.unwrap();
    let loaded = load_cached_manifest(&path).await.unwrap();

    assert_eq!(loaded.latest.release, "1.21.4");
    assert_eq!(loaded.versions.len(), 3);
}
