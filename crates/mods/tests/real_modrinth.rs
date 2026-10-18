//! Pruebas contra la API real de Modrinth: `cargo test -p mods --test real_modrinth -- --ignored --nocapture`

use mods::{InstanceTarget, ModManager, ModrinthClient, ModsService, PlannedMod};

const FABRIC_API: &str = "P7dR8mSH";
const TARGET: InstanceTarget<'static> = InstanceTarget { minecraft_version: "1.20.1", loader: "fabric" };

fn http() -> reqwest::Client {
    reqwest::Client::builder().user_agent("DisorderLauncher/0.1 (pruebas)").build().unwrap()
}

#[tokio::test]
#[ignore = "requiere internet"]
async fn search_is_filtered_by_version_and_loader() {
    let service = ModsService::new(http());
    let hits = service.search("sodium", TARGET, 5, 0).await.unwrap();
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|h| h.slug == "sodium"), "{:?}", hits.iter().map(|h| &h.slug).collect::<Vec<_>>());
}

#[tokio::test]
#[ignore = "requiere internet"]
async fn installs_fabric_api_and_reads_its_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let manager = ModManager::new(dir.path().join("game"));
    let service = ModsService::new(http());

    let installed = service.install(&manager, FABRIC_API, TARGET, 4).await.unwrap();
    assert_eq!(installed.last().unwrap().project_id, FABRIC_API);

    let list = manager.list().await.unwrap();
    let api = list.iter().find(|m| m.meta.as_ref().is_some_and(|x| x.id == "fabric-api")).expect("Fabric API instalado");
    assert!(api.enabled);
    assert!(api.source.is_some(), "debe quedar registrado el origen");

    // Instalar de nuevo no duplica ni vuelve a pedir dependencias.
    let again = service.install(&manager, FABRIC_API, TARGET, 4).await;
    println!("segunda instalacion: {:?}", again.as_ref().map(|v| v.len()));
    assert_eq!(manager.list().await.unwrap().len(), list.len());
}

#[tokio::test]
#[ignore = "requiere internet"]
async fn update_and_rollback_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let manager = ModManager::new(dir.path().join("game"));
    let service = ModsService::new(http());

    // Dejar instalada una version antigua de Fabric API.
    let client = ModrinthClient::new(http());
    let mut versions = client.project_versions(FABRIC_API, TARGET.minecraft_version, TARGET.loader).await.unwrap();
    versions.retain(|v| v.version_type == "release");
    versions.sort_by_key(|v| v.date_published);
    assert!(versions.len() >= 2, "se necesitan al menos dos versiones");
    let old = versions.first().unwrap().clone();
    let latest = versions.last().unwrap().clone();

    service
        .install_plan(
            &manager,
            vec![PlannedMod { project_id: FABRIC_API.into(), title: "Fabric API".into(), version: old.clone(), dependency_of: None }],
            2,
        )
        .await
        .unwrap();

    let updates = service.check_updates(&manager, TARGET).await.unwrap();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].latest_version, latest.version_number);

    let old_file = old.primary_file().unwrap().filename.clone();
    let updated = service.update(&manager, &old_file, TARGET).await.unwrap();
    assert_eq!(updated.version_number, latest.version_number);
    assert!(!manager.mods_dir().join(&old_file).exists() || old_file == updated.file);
    assert!(service.check_updates(&manager, TARGET).await.unwrap().is_empty());

    let entry = manager.list().await.unwrap().into_iter().find(|e| e.file_name == updated.file).unwrap();
    assert!(entry.source.unwrap().has_previous_version);

    service.rollback(&manager, &updated.file).await.unwrap();
    assert!(manager.mods_dir().join(&old_file).is_file(), "vuelve el archivo original");
    assert_eq!(service.check_updates(&manager, TARGET).await.unwrap().len(), 1, "otra vez hay actualizacion");
}

/// Forge y NeoForge: busca e instala JEI de verdad y comprueba que se lee su `mods.toml` / `neoforge.mods.toml`.
async fn install_jei_and_read_meta(target: InstanceTarget<'_>) {
    let dir = tempfile::tempdir().unwrap();
    let manager = ModManager::new(dir.path().join("game"));
    let service = ModsService::new(http());

    let hits = service.search("jei", target, 5, 0).await.unwrap();
    let jei = hits.iter().find(|h| h.slug == "jei").unwrap_or_else(|| panic!("JEI no aparece: {:?}", hits.iter().map(|h| &h.slug).collect::<Vec<_>>()));

    service.install(&manager, &jei.project_id, target, 4).await.unwrap();
    let list = manager.list().await.unwrap();
    let entry = list.iter().find(|m| m.meta.as_ref().is_some_and(|x| x.id == "jei")).unwrap_or_else(|| panic!("metadatos de JEI sin leer: {list:?}"));
    assert!(entry.enabled);
    assert!(entry.source.is_some());
}

#[tokio::test]
#[ignore = "requiere internet"]
async fn neoforge_mod_is_found_installed_and_read() {
    install_jei_and_read_meta(InstanceTarget { minecraft_version: "1.21.1", loader: "neoforge" }).await;
}

#[tokio::test]
#[ignore = "requiere internet"]
async fn forge_mod_is_found_installed_and_read() {
    install_jei_and_read_meta(InstanceTarget { minecraft_version: "1.20.1", loader: "forge" }).await;
}

/// Los mods de rendimiento recomendados se pueden pedir por su slug de Modrinth.
#[tokio::test]
#[ignore = "requiere internet"]
async fn performance_mods_install_by_slug() {
    let dir = tempfile::tempdir().unwrap();
    let manager = ModManager::new(dir.path().join("game"));
    let service = ModsService::new(http());
    for slug in ["sodium", "lithium", "ferrite-core", "entityculling", "modernfix"] {
        let installed = service.install(&manager, slug, TARGET, 4).await.unwrap_or_else(|e| panic!("{slug}: {e}"));
        println!("{slug}: {} archivo(s)", installed.len());
    }
    assert!(manager.list().await.unwrap().len() >= 5);
}
