use identity_api::{GameSessionMaterial, IdentityProviderKind, ResolvedIdentity};
use java_runtime::JavaInstallation;
use minecraft_launch::{LaunchContext, LaunchError, LaunchPlanBuilder};
use minecraft_meta::VersionDescriptor;
use std::path::Path;

const MODERN: &str = r#"{
  "id": "1.21",
  "type": "release",
  "mainClass": "net.minecraft.client.main.Main",
  "downloads": { "client": { "sha1": "aa", "size": 1, "url": "https://example.com/client.jar" } },
  "assetIndex": { "id": "17", "sha1": "bb", "size": 1, "totalSize": 1, "url": "https://example.com/17.json" },
  "assets": "17",
  "libraries": [
    { "name": "com.mojang:a:1", "downloads": { "artifact": { "path": "com/mojang/a/1/a-1.jar", "sha1": "cc", "size": 1, "url": "https://example.com/a.jar" } } },
    { "name": "org.x:win:1", "rules": [ { "action": "allow", "os": { "name": "no-such-os" } } ],
      "downloads": { "artifact": { "path": "org/x/win/1/win-1.jar", "sha1": "dd", "size": 1, "url": "https://example.com/w.jar" } } }
  ],
  "arguments": {
    "game": [
      "--username", "${auth_player_name}",
      "--uuid", "${auth_uuid}",
      "--accessToken", "${auth_access_token}",
      "--userType", "${user_type}",
      "--gameDir", "${game_directory}",
      "--assetsDir", "${assets_root}",
      "--assetIndex", "${assets_index_name}",
      "--version", "${version_name}",
      { "rules": [ { "action": "allow", "features": { "has_custom_resolution": true } } ],
        "value": [ "--width", "${resolution_width}", "--height", "${resolution_height}" ] },
      { "rules": [ { "action": "allow", "features": { "is_demo_user": true } } ], "value": "--demo" }
    ],
    "jvm": [
      "-Djava.library.path=${natives_directory}",
      "-cp", "${classpath}"
    ]
  }
}"#;

const LEGACY: &str = r#"{
  "id": "1.7.10",
  "type": "release",
  "mainClass": "net.minecraft.client.main.Main",
  "downloads": { "client": { "sha1": "aa", "size": 1, "url": "https://example.com/client.jar" } },
  "assetIndex": { "id": "1.7.10", "sha1": "bb", "size": 1, "totalSize": 1, "url": "https://example.com/i.json" },
  "assets": "1.7.10",
  "libraries": [],
  "minecraftArguments": "--username ${auth_player_name} --session ${auth_session} --version ${version_name} --gameDir ${game_directory} --userProperties ${user_properties}"
}"#;

fn identity(token: &str) -> ResolvedIdentity {
    ResolvedIdentity {
        profile_id: "p1".into(),
        provider: IdentityProviderKind::Local,
        display_name: "Steve".into(),
        game_uuid: "11111111-2222-3333-4444-555555555555".into(),
        session: GameSessionMaterial { access_token: token.into(), token_type: "legacy".into() },
        skin: None,
    }
}

fn java() -> JavaInstallation {
    JavaInstallation { path: "/jre/21".into(), version: "21.0.2".into(), major: 21, managed: true }
}

fn ctx() -> LaunchContext {
    LaunchContext::new("/data", "/data/instances/i1/game")
}

fn arg_after<'a>(args: &'a [String], flag: &str) -> &'a str {
    let i = args.iter().position(|a| a == flag).unwrap_or_else(|| panic!("falta {flag}"));
    &args[i + 1]
}

#[test]
fn modern_version_substitutes_identity_and_paths() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();

    assert_eq!(plan.main_class, "net.minecraft.client.main.Main");
    assert_eq!(arg_after(&plan.game_args, "--username"), "Steve");
    assert_eq!(arg_after(&plan.game_args, "--uuid"), "11111111-2222-3333-4444-555555555555");
    assert_eq!(arg_after(&plan.game_args, "--userType"), "legacy");
    assert_eq!(arg_after(&plan.game_args, "--assetIndex"), "17");
    assert_eq!(arg_after(&plan.game_args, "--version"), "1.21");
    assert!(Path::new(arg_after(&plan.game_args, "--gameDir")).ends_with("game"));
    assert_eq!(plan.working_directory, Path::new("/data/instances/i1/game"));
    assert!(plan.java_path.ends_with(if cfg!(windows) { "java.exe" } else { "java" }));
}

#[test]
fn classpath_applies_rules_and_ends_with_client_jar() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();

    assert_eq!(plan.classpath.len(), 2, "la libreria de otro SO no entra");
    assert!(plan.classpath[0].ends_with("a-1.jar"));
    assert!(plan.classpath[1].ends_with("1.21.jar"));
}

#[test]
fn classpath_is_emitted_once_by_command_args() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();
    let args = plan.command_args().unwrap();

    assert_eq!(args.iter().filter(|a| *a == "-cp").count(), 1);
    assert!(!args.iter().any(|a| a.contains("${")));
    let main_pos = args.iter().position(|a| a == &plan.main_class).unwrap();
    assert_eq!(args[main_pos - 2], "-cp");
    assert_eq!(args[main_pos + 1], "--username");
}

#[test]
fn feature_arguments_follow_context() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();

    let plain = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();
    assert!(!plain.game_args.contains(&"--width".to_string()));
    assert!(!plain.game_args.contains(&"--demo".to_string()));

    let mut c = ctx();
    c.window_size = Some((1280, 720));
    let sized = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &c).unwrap();
    assert_eq!(arg_after(&sized.game_args, "--width"), "1280");
    assert_eq!(arg_after(&sized.game_args, "--height"), "720");
}

#[test]
fn memory_settings_become_jvm_flags() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let mut c = ctx();
    c.memory.min_mb = Some(512);
    c.memory.max_mb = Some(4096);
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &c).unwrap();

    assert!(plan.jvm_args.contains(&"-Xms512M".to_string()));
    assert!(plan.jvm_args.contains(&"-Xmx4096M".to_string()));
}

#[test]
fn legacy_arguments_get_default_jvm_args() {
    let version: VersionDescriptor = serde_json::from_str(LEGACY).unwrap();
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();

    assert!(plan.jvm_args.iter().any(|a| a.starts_with("-Djava.library.path=")));
    assert_eq!(arg_after(&plan.game_args, "--session"), "0");
    assert_eq!(arg_after(&plan.game_args, "--userProperties"), "{}");
    assert_eq!(plan.classpath.len(), 1);
}

#[test]
fn debug_output_never_contains_the_access_token() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let id = identity("tok-very-secret-123");
    let plan = LaunchPlanBuilder::build(&version, &id, &java(), &ctx()).unwrap();

    assert!(plan.game_args.contains(&"tok-very-secret-123".to_string()));
    assert!(!format!("{plan:?}").contains("tok-very-secret-123"));
    assert!(!format!("{id:?}").contains("tok-very-secret-123"));
    assert_eq!(plan.secrets(), ["tok-very-secret-123"]);
}

#[test]
fn short_offline_token_is_not_registered_as_secret() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap();
    assert!(plan.secrets().is_empty());
}

#[test]
fn unknown_placeholder_fails_instead_of_launching_broken() {
    let broken = MODERN.replace("${assets_index_name}", "${no_such_thing}");
    let version: VersionDescriptor = serde_json::from_str(&broken).unwrap();
    let err = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &ctx()).unwrap_err();
    assert!(matches!(err, LaunchError::UnresolvedPlaceholder(k) if k == "no_such_thing"));
}

#[test]
fn validate_reports_missing_java() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let c = LaunchContext::new(dir.path(), dir.path().join("game"));
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java(), &c).unwrap();

    assert!(matches!(plan.validate(), Err(LaunchError::JavaMissing(_))));
}

#[test]
fn validate_lists_missing_classpath_entries_when_java_exists() {
    let version: VersionDescriptor = serde_json::from_str(MODERN).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let jre = dir.path().join("jre");
    let bin = jre.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let exe = bin.join(if cfg!(windows) { "java.exe" } else { "java" });
    std::fs::write(&exe, b"").unwrap();

    let java = JavaInstallation { path: jre, version: "21".into(), major: 21, managed: true };
    let c = LaunchContext::new(dir.path(), dir.path().join("game"));
    let plan = LaunchPlanBuilder::build(&version, &identity("0"), &java, &c).unwrap();

    match plan.validate() {
        Err(LaunchError::MissingFiles(files)) => assert_eq!(files.len(), 2),
        other => panic!("esperaba MissingFiles, obtuve {other:?}"),
    }
}
