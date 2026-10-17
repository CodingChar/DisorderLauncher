use minecraft_install::*;
use minecraft_meta::VersionDescriptor;
use std::io::Write;

fn legacy_version(os_key: &str) -> VersionDescriptor {
    let json = format!(
        r#"{{
  "id": "1.12.2", "type": "release", "mainClass": "net.minecraft.client.main.Main",
  "downloads": {{ "client": {{ "sha1": "aa", "size": 1, "url": "https://x/c.jar" }} }},
  "assetIndex": {{ "id": "1.12", "sha1": "bb", "size": 1, "totalSize": 1, "url": "https://x/i.json" }},
  "assets": "1.12",
  "minecraftArguments": "--username ${{auth_player_name}}",
  "libraries": [
    {{ "name": "org.lwjgl:lwjgl-platform:2.9.4",
       "natives": {{ "{os_key}": "natives-${{arch}}-test" }},
       "extract": {{ "exclude": ["META-INF/"] }},
       "downloads": {{
         "artifact": {{ "path": "org/lwjgl/lwjgl-platform-2.9.4.jar", "sha1": "cc", "size": 1, "url": "https://x/p.jar" }},
         "classifiers": {{ "natives-{bits}-test": {{ "path": "org/lwjgl/lwjgl-platform-2.9.4-natives.jar", "sha1": "dd", "size": 1, "url": "https://x/n.jar" }} }}
       }} }}
  ]
}}"#,
        bits = if cfg!(target_pointer_width = "64") { "64" } else { "32" }
    );
    serde_json::from_str(&json).unwrap()
}

fn current_os() -> &'static str {
    if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "osx" } else { "linux" }
}

#[test]
fn natives_are_downloaded_to_libraries_and_resolve_arch_placeholder() {
    let plan = build_install_plan(&legacy_version(current_os()), std::path::Path::new("/data"));

    assert_eq!(plan.natives.len(), 1);
    assert!(plan.natives[0].path.ends_with("org/lwjgl/lwjgl-platform-2.9.4-natives.jar"));
    assert!(plan.natives[0].path.starts_with("/data/libraries"));
    assert_eq!(plan.native_extractions[0].exclude, vec!["META-INF/".to_string()]);
    assert!(plan.natives_dir.ends_with("natives"));
    // El jar de natives no entra en el classpath de libraries normales.
    assert_eq!(plan.libraries.len(), 1);
}

#[test]
fn library_without_native_for_this_os_has_no_natives() {
    let plan = build_install_plan(&legacy_version("some-other-os"), std::path::Path::new("/data"));
    assert!(plan.natives.is_empty());
}

#[tokio::test]
async fn extract_natives_unpacks_with_exclusions() {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("n.jar");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for (name, data) in [("lwjgl64.dll", &b"dll"[..]), ("META-INF/MANIFEST.MF", b"m")] {
        writer.start_file(name, opts).unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap();

    let mut plan = build_install_plan(&legacy_version(current_os()), dir.path());
    plan.natives_dir = dir.path().join("natives");
    plan.native_extractions = vec![NativeExtraction { jar, exclude: vec!["META-INF/".into()] }];

    extract_natives(&plan).await.unwrap();
    assert!(plan.natives_dir.join("lwjgl64.dll").is_file());
    assert!(!plan.natives_dir.join("META-INF").exists());
}

#[tokio::test]
async fn malicious_native_jar_fails_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let jar = dir.path().join("evil.jar");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    writer.start_file("../../escape.dll", zip::write::SimpleFileOptions::default()).unwrap();
    writer.write_all(b"x").unwrap();
    writer.finish().unwrap();

    let mut plan = build_install_plan(&legacy_version(current_os()), dir.path());
    plan.natives_dir = dir.path().join("a").join("natives");
    plan.native_extractions = vec![NativeExtraction { jar, exclude: vec![] }];

    assert!(extract_natives(&plan).await.is_err());
    assert!(!dir.path().join("escape.dll").exists());
}
