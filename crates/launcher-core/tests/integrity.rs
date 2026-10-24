//! Que hace el launcher cuando algo se borra o se corta desde fuera.

use instance_manager::LoaderKind;
use launcher_core::{
    GameOutputEvent, InstallProgressEvent, IssueKind, LaunchStateEvent, Launcher, LauncherError, LauncherEvents,
    LauncherPaths, OutputStream,
};
use std::sync::Arc;

struct Silent;
impl LauncherEvents for Silent {
    fn install_progress(&self, _: InstallProgressEvent) {}
    fn launch_state(&self, _: LaunchStateEvent) {}
    fn game_output(&self, _: OutputStream, _: GameOutputEvent) {}
}

fn launcher() -> (tempfile::TempDir, Launcher) {
    let dir = tempfile::tempdir().unwrap();
    let launcher = Launcher::new(LauncherPaths::new(dir.path()), Arc::new(Silent))
        .with_manifest_url("http://127.0.0.1:1/version_manifest_v2.json");
    (dir, launcher)
}

#[tokio::test]
async fn an_instance_deleted_from_outside_disappears_from_the_list_and_cannot_launch() {
    let (_d, launcher) = launcher();
    launcher.accounts_create_local("David").await.unwrap();
    let keep = launcher.instances_create("Queda", "1.21", LoaderKind::Vanilla, None).await.unwrap();
    let gone = launcher.instances_create("Se borra", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    // Borrado desde el explorador de archivos.
    std::fs::remove_dir_all(gone.game_dir.parent().unwrap()).unwrap();

    let ids: Vec<_> = launcher.instances_list().await.unwrap().into_iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![keep.id.clone()]);
    assert!(matches!(
        Arc::new(launcher).launch(&gone.id).await,
        Err(LauncherError::InstanceNotFound)
    ));
}

#[tokio::test]
async fn missing_internal_folders_are_recreated_and_leftovers_are_cleaned() {
    let (dir, launcher) = launcher();
    launcher.accounts_create_local("David").await.unwrap();
    let instance = launcher.instances_create("Mi mundo", "1.21", LoaderKind::Vanilla, None).await.unwrap();
    let root = dir.path().join("instances");

    std::fs::remove_dir_all(&instance.game_dir).unwrap();
    // Resto de un borrado incompleto: carpeta con forma de instancia pero sin ficha.
    let leftover = root.join("inst_00000000000000000000000000000000");
    std::fs::create_dir_all(leftover.join("game/mods")).unwrap();

    let report = launcher.instances_integrity().await.unwrap();
    let kinds: Vec<_> = report.issues.iter().map(|i| (i.kind, i.fixed)).collect();
    assert!(kinds.contains(&(IssueKind::LayoutRepaired, true)), "{kinds:?}");
    assert!(kinds.contains(&(IssueKind::LeftoverRemoved, true)), "{kinds:?}");
    assert!(report.needs_attention().next().is_none());
    assert!(instance.game_dir.join("mods").is_dir());
    assert!(!leftover.exists());

    // Una segunda revision ya no encuentra nada.
    assert!(launcher.instances_integrity().await.unwrap().issues.is_empty());
}

#[tokio::test]
async fn a_cut_modpack_import_is_rolled_back_and_blocks_launching_meanwhile() {
    let (_d, launcher) = launcher();
    launcher.accounts_create_local("David").await.unwrap();
    let instance = launcher.instances_create("Pack", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    // El launcher se cerro a mitad de una importacion: queda la marca en disco.
    std::fs::create_dir_all(instance.game_dir.join(".disorder")).unwrap();
    std::fs::write(instance.game_dir.join(".disorder/import.pending"), b"importando").unwrap();

    let launcher = Arc::new(launcher);
    assert!(matches!(launcher.launch(&instance.id).await, Err(LauncherError::InvalidInput(_))));

    let report = launcher.instances_integrity().await.unwrap();
    assert!(report.issues.iter().any(|i| i.kind == IssueKind::ImportRolledBack && i.fixed));
    assert!(launcher.instances_list().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_removed_profile_is_reported_never_replaced_silently() {
    let (dir, launcher) = launcher();
    let account = launcher.accounts_create_local("David").await.unwrap();
    let instance = launcher.instances_create("Mi mundo", "1.21", LoaderKind::Vanilla, None).await.unwrap();

    // El archivo del perfil se borra a mano.
    std::fs::remove_file(dir.path().join("accounts").join(format!("{}.json", account.id))).unwrap();

    let report = launcher.instances_integrity().await.unwrap();
    let issue = report.needs_attention().next().expect("pide una decision");
    assert_eq!((issue.instance_id.as_str(), issue.kind), (instance.id.as_str(), IssueKind::ProfileMissing));
    // La instancia sigue apuntando al perfil que tenia.
    let still = launcher.instances_list().await.unwrap();
    assert_eq!(still[0].profile_id, account.id);
}
