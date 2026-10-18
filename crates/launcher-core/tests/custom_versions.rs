//! Versiones instaladas fuera del launcher («Personalizado») y compatibilidad de modpacks.

use instance_manager::LoaderKind;
use launcher_core::{
    GameOutputEvent, InstallProgressEvent, Launcher, LauncherError, LauncherEvents, LauncherPaths, LaunchStateEvent,
    OutputStream,
};
use minecraft_meta::VersionDescriptor;
use std::sync::Arc;

/// Puerto cerrado: cualquier descarga falla de inmediato, simulando falta de red.
const NO_NETWORK: &str = "http://127.0.0.1:1/version_manifest_v2.json";

struct Quiet;

impl LauncherEvents for Quiet {
    fn install_progress(&self, _: InstallProgressEvent) {}
    fn launch_state(&self, _: LaunchStateEvent) {}
    fn game_output(&self, _: OutputStream, _: GameOutputEvent) {}
}

fn launcher() -> (tempfile::TempDir, Arc<Launcher>) {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Quiet)).with_manifest_url(NO_NETWORK);
    (dir, Arc::new(launcher))
}

/// Un `.minecraft` del launcher oficial con OptiFine 1.12.2 puesto a mano.
fn external_minecraft() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let put = |path: &str, body: &str| {
        let p = dir.path().join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    };
    put(
        "versions/1.12.2/1.12.2.json",
        r#"{"id":"1.12.2","type":"release","releaseTime":"2017-09-18T08:39:46+00:00","mainClass":"net.minecraft.client.main.Main",
        "minecraftArguments":"--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory}",
        "assets":"1.12","assetIndex":{"id":"1.12","sha1":"aa","size":1,"totalSize":1,"url":"https://example.com/i.json"},
        "downloads":{"client":{"sha1":"5b0aeb5e1e1c8ba2d9e1a4d1d9a8e6c9c1f2a3b4","size":3,"url":"https://example.com/client.jar"}},
        "javaVersion":{"component":"jre-legacy","majorVersion":8},
        "libraries":[{"name":"net.minecraft:launchwrapper:1.12","downloads":{"artifact":{"path":"net/minecraft/launchwrapper/1.12/launchwrapper-1.12.jar","sha1":"b","size":1,"url":"https://example.com/lw.jar"}}}]}"#,
    );
    put("versions/1.12.2/1.12.2.jar", "cliente vanilla");
    put(
        "versions/1.12.2-OptiFine_HD_U_E3/1.12.2-OptiFine_HD_U_E3.json",
        r#"{"id":"1.12.2-OptiFine_HD_U_E3","inheritsFrom":"1.12.2","type":"release","releaseTime":"2018-01-01T00:00:00+00:00",
        "mainClass":"net.minecraft.launchwrapper.Launch",
        "minecraftArguments":"--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --tweakClass optifine.OptiFineTweaker",
        "libraries":[{"name":"optifine:OptiFine:1.12.2_HD_U_E3"},{"name":"optifine:launchwrapper-of:2.1"}]}"#,
    );
    put("libraries/optifine/OptiFine/1.12.2_HD_U_E3/OptiFine-1.12.2_HD_U_E3.jar", "optifine");
    put("libraries/optifine/launchwrapper-of/2.1/launchwrapper-of-2.1.jar", "wrapper");
    put("saves/Mi mundo/level.dat", "datos");
    put("mods/mod.jar", "mod");
    dir
}

#[tokio::test]
async fn a_custom_version_from_dot_minecraft_becomes_a_normal_instance() {
    let (_data, launcher) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let external = external_minecraft();

    let scan = launcher.external_versions_scan(Some(external.path())).await.unwrap();
    assert!(scan.found);
    assert_eq!(scan.versions.len(), 1, "la 1.12.2 normal ya esta en la lista oficial");
    assert_eq!(scan.versions[0].id, "1.12.2-OptiFine_HD_U_E3");
    assert!(!scan.versions[0].imported);

    let instance = launcher
        .instances_create_custom("Mi OptiFine", external.path(), "1.12.2-OptiFine_HD_U_E3", true)
        .await
        .unwrap();
    assert_eq!(instance.minecraft_version, "custom-1.12.2-OptiFine_HD_U_E3");
    assert_eq!(instance.loader, LoaderKind::Vanilla);

    // Mundos y mods se copian; el origen queda intacto.
    assert!(instance.game_dir.join("saves/Mi mundo/level.dat").is_file());
    assert!(instance.game_dir.join("mods/mod.jar").is_file());
    assert!(external.path().join("saves/Mi mundo/level.dat").is_file());

    // El perfil queda fusionado con su version base y las librerias locales, copiadas.
    let paths = launcher.paths();
    let descriptor: VersionDescriptor =
        serde_json::from_slice(&std::fs::read(paths.version_descriptor(&instance.minecraft_version)).unwrap()).unwrap();
    assert_eq!(descriptor.id, "custom-1.12.2-OptiFine_HD_U_E3");
    assert_eq!(descriptor.inherits_from.as_deref(), Some("1.12.2"));
    assert_eq!(descriptor.release_time.as_deref(), Some("2017-09-18T08:39:46+00:00"), "la fecha es la de la version base");
    assert_eq!(descriptor.main_class, "net.minecraft.launchwrapper.Launch");
    assert_eq!(descriptor.libraries.len(), 3);
    let libs = paths.minecraft_dir().join("libraries");
    assert!(libs.join("optifine/OptiFine/1.12.2_HD_U_E3/OptiFine-1.12.2_HD_U_E3.jar").is_file());
    let jar = paths.minecraft_dir().join("versions").join(&descriptor.id).join(format!("{}.jar", descriptor.id));
    assert_eq!(std::fs::read(jar).unwrap(), b"cliente vanilla");

    // El motor de lanzamiento no necesita nada especial: OptiFine entra en el classpath.
    use minecraft_launch::{LaunchContext, LaunchPlanBuilder};
    let identity = identity_api::ResolvedIdentity {
        profile_id: "p".into(),
        provider: identity_api::IdentityProviderKind::Local,
        display_name: "Steve".into(),
        game_uuid: "11111111-2222-3333-4444-555555555555".into(),
        session: identity_api::GameSessionMaterial { access_token: "0".into(), token_type: "legacy".into() },
        skin: None,
    };
    let java = java_runtime::JavaInstallation { path: "/jre".into(), version: "8".into(), major: 8, managed: false };
    let plan =
        LaunchPlanBuilder::build(&descriptor, &identity, &java, &LaunchContext::new(paths.minecraft_dir(), &instance.game_dir)).unwrap();
    let classpath: Vec<String> = plan.classpath.iter().map(|p| p.to_string_lossy().replace('\\', "/")).collect();
    assert!(classpath.iter().any(|p| p.ends_with("optifine/OptiFine/1.12.2_HD_U_E3/OptiFine-1.12.2_HD_U_E3.jar")), "{classpath:?}");
    assert!(plan.game_args.iter().any(|a| a == "optifine.OptiFineTweaker"));

    // Ya aparece como importada, y volver a crearla no rompe nada.
    let again = launcher.external_versions_scan(Some(external.path())).await.unwrap();
    assert!(again.versions[0].imported);
    launcher.instances_create_custom("Otra", external.path(), "1.12.2-OptiFine_HD_U_E3", false).await.unwrap();
}

#[tokio::test]
async fn a_custom_version_with_missing_files_or_unsafe_names_is_refused() {
    let (_data, launcher) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let external = external_minecraft();
    std::fs::remove_file(external.path().join("libraries/optifine/launchwrapper-of/2.1/launchwrapper-of-2.1.jar")).unwrap();

    let missing = launcher.instances_create_custom("X", external.path(), "1.12.2-OptiFine_HD_U_E3", false).await;
    assert!(matches!(missing, Err(LauncherError::InvalidInput(m)) if m.contains("launchwrapper-of")));
    assert!(launcher.instances_list().await.unwrap().is_empty(), "no queda una instancia a medias");

    for bad in ["../1.12.2", "a/b", ""] {
        let result = launcher.instances_create_custom("X", external.path(), bad, false).await;
        assert!(matches!(result, Err(LauncherError::InvalidInput(_))), "{bad}");
    }
}

#[tokio::test]
async fn the_scan_is_harmless_when_the_folder_does_not_exist() {
    let (data, launcher) = launcher();
    let scan = launcher.external_versions_scan(Some(&data.path().join("no-existe"))).await.unwrap();
    assert!(!scan.found);
    assert!(scan.versions.is_empty());
}

#[tokio::test]
async fn an_official_version_name_is_never_overwritten_by_an_import() {
    let (_data, launcher) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    let external = external_minecraft();
    // Alguien dejo una version del launcher con el nombre que tendria la importada, sin ser una importacion.
    let existing = launcher.paths().version_descriptor("custom-1.12.2-OptiFine_HD_U_E3");
    std::fs::create_dir_all(existing.parent().unwrap()).unwrap();
    std::fs::write(&existing, r#"{"id":"custom-1.12.2-OptiFine_HD_U_E3"}"#).unwrap();
    let result = launcher.instances_create_custom("X", external.path(), "1.12.2-OptiFine_HD_U_E3", false).await;
    assert!(matches!(result, Err(LauncherError::InvalidInput(_))));
    assert_eq!(std::fs::read_to_string(&existing).unwrap(), r#"{"id":"custom-1.12.2-OptiFine_HD_U_E3"}"#);
}

#[tokio::test]
async fn curseforge_and_prism_packs_get_an_explanation_instead_of_a_generic_error() {
    use std::io::Write;
    let (data, launcher) = launcher();
    launcher.accounts_create_local("Steve").await.unwrap();
    for (name, body, expected) in [
        ("manifest.json", r#"{"manifestType":"minecraftModpack"}"#, "CurseForge"),
        ("mmc-pack.json", "{}", "MultiMC"),
    ] {
        let path = data.path().join(format!("{expected}.zip"));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
        zip.finish().unwrap();
        let err = launcher.modpack_import(&path).await.unwrap_err();
        assert!(matches!(&err, LauncherError::InvalidInput(m) if m.contains(expected) && m.contains("mrpack")), "{err}");
    }
}
