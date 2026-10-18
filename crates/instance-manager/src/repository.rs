use crate::file::InstanceFile;
use crate::model::{validate_icon, validate_memory, validate_name, validate_required, validate_version};
use crate::{
    Instance, InstanceError, InstanceUpdate, JavaSelection, MemoryPolicy, NewInstance, Optimization,
};
use chrono::Utc;
use std::path::{Path, PathBuf};
use tokio::fs;

const ID_PREFIX: &str = "inst_";
/// Subcarpetas que se crean dentro de `game/` para que el usuario no tenga que tocar el filesystem.
const GAME_SUBDIRS: &[&str] = &["mods", "config", "saves", "resourcepacks", "shaderpacks"];

#[async_trait::async_trait]
pub trait InstanceRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Instance>, InstanceError>;
    async fn get(&self, id: &str) -> Result<Option<Instance>, InstanceError>;
    async fn create(&self, new: NewInstance) -> Result<Instance, InstanceError>;
    async fn update(&self, id: &str, changes: InstanceUpdate) -> Result<Instance, InstanceError>;
    async fn delete(&self, id: &str) -> Result<(), InstanceError>;
    async fn duplicate(&self, id: &str) -> Result<Instance, InstanceError>;
}

/// Persistencia en `<root>/<id>/instance.json` + `game/` + `logs/` (spec 06).
pub struct FileInstanceRepository {
    root: PathBuf,
}

impl FileInstanceRepository {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    pub fn instance_dir(&self, id: &str) -> Result<PathBuf, InstanceError> {
        validate_id(id)?;
        Ok(self.root.join(id))
    }

    pub fn logs_dir(&self, id: &str) -> Result<PathBuf, InstanceError> {
        Ok(self.instance_dir(id)?.join("logs"))
    }

    async fn read(&self, id: &str) -> Result<Option<Instance>, InstanceError> {
        let dir = self.instance_dir(id)?;
        let path = dir.join("instance.json");
        let data = match fs::read_to_string(&path).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let file: InstanceFile = serde_json::from_str(&data)?;
        if file.id != id {
            return Err(InstanceError::Invalid("el id no coincide con su carpeta".into()));
        }
        Ok(Some(file.into_instance(dir.join("game"))?))
    }

    async fn write(&self, instance: &Instance) -> Result<(), InstanceError> {
        let dir = self.instance_dir(&instance.id)?;
        fs::create_dir_all(&dir).await?;
        let path = dir.join("instance.json");
        let part = dir.join("instance.json.part");
        let data = serde_json::to_string_pretty(&InstanceFile::from_instance(instance))?;
        fs::write(&part, data).await?;
        fs::rename(&part, &path).await?;
        Ok(())
    }

    async fn create_layout(&self, id: &str) -> Result<PathBuf, InstanceError> {
        let dir = self.instance_dir(id)?;
        let game = dir.join("game");
        for sub in GAME_SUBDIRS {
            fs::create_dir_all(game.join(sub)).await?;
        }
        fs::create_dir_all(dir.join("logs")).await?;
        Ok(game)
    }
}

#[async_trait::async_trait]
impl InstanceRepository for FileInstanceRepository {
    async fn list(&self) -> Result<Vec<Instance>, InstanceError> {
        let mut entries = match fs::read_dir(&self.root).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut instances = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if validate_id(&name).is_err() {
                continue;
            }
            // Una instancia danada no debe esconder las demas.
            match self.read(&name).await {
                Ok(Some(instance)) => instances.push(instance),
                Ok(None) => {}
                Err(e) => tracing::warn!(instance = %name, error = %e, "instancia ilegible, se omite"),
            }
        }
        instances.sort_by_key(|i| i.created_at);
        Ok(instances)
    }

    async fn get(&self, id: &str) -> Result<Option<Instance>, InstanceError> {
        self.read(id).await
    }

    async fn create(&self, new: NewInstance) -> Result<Instance, InstanceError> {
        let name = validate_name(&new.name)?;
        let minecraft_version = validate_version(&new.minecraft_version)?;
        validate_required("el perfil", &new.profile_id)?;

        let id = format!("{ID_PREFIX}{}", uuid::Uuid::new_v4().simple());
        let game_dir = self.create_layout(&id).await?;
        let now = Utc::now();
        let instance = Instance {
            id,
            name,
            minecraft_version,
            loader: new.loader,
            loader_version: new.loader_version,
            profile_id: new.profile_id,
            java: JavaSelection::Auto,
            memory: MemoryPolicy::Auto,
            icon: None,
            optimization: Optimization::default(),
            game_dir,
            created_at: now,
            updated_at: now,
        };
        self.write(&instance).await?;
        Ok(instance)
    }

    async fn update(&self, id: &str, changes: InstanceUpdate) -> Result<Instance, InstanceError> {
        let mut instance = self
            .read(id)
            .await?
            .ok_or_else(|| InstanceError::NotFound(id.to_string()))?;

        if let Some(name) = changes.name {
            instance.name = validate_name(&name)?;
        }
        if let Some(version) = changes.minecraft_version {
            instance.minecraft_version = validate_version(&version)?;
        }
        if let Some(profile_id) = changes.profile_id {
            validate_required("el perfil", &profile_id)?;
            instance.profile_id = profile_id;
        }
        if let Some(java) = changes.java {
            instance.java = java;
        }
        if let Some(memory) = changes.memory {
            validate_memory(&memory)?;
            instance.memory = memory;
        }
        if let Some(optimization) = changes.optimization {
            instance.optimization = optimization;
        }
        if let Some(icon) = changes.icon {
            instance.icon = validate_icon(&icon)?;
        }
        instance.updated_at = Utc::now();
        self.write(&instance).await?;
        Ok(instance)
    }

    async fn delete(&self, id: &str) -> Result<(), InstanceError> {
        let dir = self.instance_dir(id)?;
        match fs::remove_dir_all(&dir).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    async fn duplicate(&self, id: &str) -> Result<Instance, InstanceError> {
        let source = self
            .read(id)
            .await?
            .ok_or_else(|| InstanceError::NotFound(id.to_string()))?;

        let new_id = format!("{ID_PREFIX}{}", uuid::Uuid::new_v4().simple());
        let new_game = self.create_layout(&new_id).await?;

        let from = source.game_dir.clone();
        let to = new_game.clone();
        let copied = tokio::task::spawn_blocking(move || copy_dir(&from, &to))
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        if let Err(e) = copied {
            let _ = fs::remove_dir_all(self.instance_dir(&new_id)?).await;
            return Err(e.into());
        }

        let now = Utc::now();
        let suffix = " (copia)";
        let base: String = source
            .name
            .chars()
            .take(crate::MAX_NAME_LEN - suffix.chars().count())
            .collect();
        let copy = Instance {
            id: new_id,
            name: format!("{base}{suffix}"),
            game_dir: new_game,
            created_at: now,
            updated_at: now,
            ..source
        };
        self.write(&copy).await?;
        Ok(copy)
    }
}

/// Los ids se usan como nombre de carpeta: solo `inst_` + alfanumericos.
fn validate_id(id: &str) -> Result<(), InstanceError> {
    let body = id.strip_prefix(ID_PREFIX).ok_or(InstanceError::InvalidId)?;
    if body.is_empty() || body.len() > 64 || !body.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(InstanceError::InvalidId);
    }
    Ok(())
}

/// Copia recursiva. Ignora enlaces simbolicos para no escapar del directorio de la instancia.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LoaderKind;

    fn new_instance(name: &str) -> NewInstance {
        NewInstance {
            name: name.into(),
            minecraft_version: "1.21".into(),
            loader: LoaderKind::Vanilla,
            loader_version: None,
            profile_id: "acc_local_1".into(),
        }
    }

    fn repo() -> (tempfile::TempDir, FileInstanceRepository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = FileInstanceRepository::new(dir.path().join("instances"));
        (dir, repo)
    }

    #[tokio::test]
    async fn create_builds_layout_and_persists() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("  Mi mundo  ")).await.unwrap();

        assert_eq!(inst.name, "Mi mundo");
        assert!(inst.id.starts_with("inst_"));
        assert!(inst.game_dir.join("mods").is_dir());
        assert!(inst.game_dir.join("saves").is_dir());
        assert!(repo.logs_dir(&inst.id).unwrap().is_dir());

        let loaded = repo.get(&inst.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "Mi mundo");
        assert_eq!(loaded.game_dir, inst.game_dir);
    }

    #[tokio::test]
    async fn instance_json_matches_spec_and_has_no_secrets() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        let raw = std::fs::read_to_string(repo.instance_dir(&inst.id).unwrap().join("instance.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["loader"]["kind"], "vanilla");
        assert_eq!(json["profileId"], "acc_local_1");
        assert_eq!(json["java"]["mode"], "auto");
        assert_eq!(json["memory"]["mode"], "auto");
        assert!(json.get("gameDir").is_none(), "no se guardan rutas absolutas");
        assert!(!raw.to_lowercase().contains("token"));
    }

    #[tokio::test]
    async fn list_is_ordered_and_skips_corrupt_instances() {
        let (_d, repo) = repo();
        let a = repo.create(new_instance("A")).await.unwrap();
        let b = repo.create(new_instance("B")).await.unwrap();
        let bad = repo.instance_dir("inst_broken").unwrap();
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("instance.json"), "{ nope").unwrap();
        std::fs::create_dir_all(repo.root.join("not-an-instance")).unwrap();

        let ids: Vec<_> = repo.list().await.unwrap().into_iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[tokio::test]
    async fn list_without_root_is_empty() {
        let (_d, repo) = repo();
        assert!(repo.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn update_changes_only_requested_fields() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        let updated = repo
            .update(
                &inst.id,
                InstanceUpdate {
                    name: Some("Nuevo".into()),
                    profile_id: Some("acc_ms_9".into()),
                    memory: Some(MemoryPolicy::Fixed { min_mb: 512, max_mb: 4096 }),
                    java: Some(JavaSelection::Path { path: "/opt/jdk".into() }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "Nuevo");
        assert_eq!(updated.minecraft_version, "1.21");
        assert!(updated.updated_at >= inst.updated_at);

        let reloaded = repo.get(&inst.id).await.unwrap().unwrap();
        assert_eq!(reloaded.profile_id, "acc_ms_9");
        assert_eq!(reloaded.memory, MemoryPolicy::Fixed { min_mb: 512, max_mb: 4096 });
        assert_eq!(reloaded.java, JavaSelection::Path { path: "/opt/jdk".into() });
    }

    #[tokio::test]
    async fn validation_rejects_bad_input() {
        let (_d, repo) = repo();
        assert!(matches!(repo.create(new_instance("   ")).await, Err(InstanceError::Invalid(_))));
        assert!(matches!(repo.create(new_instance(&"x".repeat(65))).await, Err(InstanceError::Invalid(_))));
        assert!(matches!(repo.create(new_instance("a\nb")).await, Err(InstanceError::Invalid(_))));

        let inst = repo.create(new_instance("A")).await.unwrap();
        let bad_mem = InstanceUpdate {
            memory: Some(MemoryPolicy::Fixed { min_mb: 4096, max_mb: 512 }),
            ..Default::default()
        };
        assert!(matches!(repo.update(&inst.id, bad_mem).await, Err(InstanceError::Invalid(_))));
    }

    #[tokio::test]
    async fn icon_is_a_short_id_that_can_be_set_cleared_and_is_copied() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        assert_eq!(inst.icon, None);

        let set = repo.update(&inst.id, InstanceUpdate { icon: Some("diamond-sword".into()), ..Default::default() }).await.unwrap();
        assert_eq!(set.icon.as_deref(), Some("diamond-sword"));
        assert_eq!(repo.duplicate(&inst.id).await.unwrap().icon.as_deref(), Some("diamond-sword"));
        // Sin tocar el icono, otros cambios lo conservan.
        let renamed = repo.update(&inst.id, InstanceUpdate { name: Some("B".into()), ..Default::default() }).await.unwrap();
        assert_eq!(renamed.icon.as_deref(), Some("diamond-sword"));

        for bad in ["../x", "A B", "Diamond", "a/b", &"x".repeat(33)] {
            let r = repo.update(&inst.id, InstanceUpdate { icon: Some(bad.into()), ..Default::default() }).await;
            assert!(matches!(r, Err(InstanceError::Invalid(_))), "{bad}");
        }
        let cleared = repo.update(&inst.id, InstanceUpdate { icon: Some(String::new()), ..Default::default() }).await.unwrap();
        assert_eq!(cleared.icon, None);
    }

    #[tokio::test]
    async fn minecraft_versions_cannot_be_paths() {
        let (_d, repo) = repo();
        for bad in ["../../x", "1.21/../x", r"a\b", ".hidden", "", "a/b", "v?x", &"9".repeat(65)] {
            let mut new = new_instance("A");
            new.minecraft_version = bad.to_string();
            assert!(matches!(repo.create(new).await, Err(InstanceError::Invalid(_))), "{bad}");
        }
        for ok in ["1.21.4", "24w14a", "b1.7.3", "3D Shareware v1.34", "1.14.4-pre1", "rd-132211"] {
            let mut new = new_instance("A");
            new.minecraft_version = ok.to_string();
            assert!(repo.create(new).await.is_ok(), "{ok}");
        }
        let inst = repo.create(new_instance("B")).await.unwrap();
        let bad = InstanceUpdate { minecraft_version: Some("../x".into()), ..Default::default() };
        assert!(matches!(repo.update(&inst.id, bad).await, Err(InstanceError::Invalid(_))));
    }

    #[tokio::test]
    async fn update_missing_instance_is_not_found() {
        let (_d, repo) = repo();
        let err = repo.update("inst_abc", InstanceUpdate::default()).await.unwrap_err();
        assert!(matches!(err, InstanceError::NotFound(_)));
    }

    #[tokio::test]
    async fn ids_cannot_escape_the_root() {
        let (_d, repo) = repo();
        for id in ["../x", "inst_../x", "inst_a/b", "inst_a\\b", "", "inst_", "other"] {
            assert!(matches!(repo.get(id).await, Err(InstanceError::InvalidId)), "{id}");
            assert!(matches!(repo.delete(id).await, Err(InstanceError::InvalidId)), "{id}");
        }
    }

    #[tokio::test]
    async fn delete_removes_everything_and_is_idempotent() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        repo.delete(&inst.id).await.unwrap();
        assert!(repo.get(&inst.id).await.unwrap().is_none());
        assert!(!repo.instance_dir(&inst.id).unwrap().exists());
        repo.delete(&inst.id).await.unwrap();
    }

    #[tokio::test]
    async fn duplicate_copies_game_files_with_new_identity() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("Original")).await.unwrap();
        std::fs::write(inst.game_dir.join("mods").join("a.jar"), b"jar").unwrap();
        std::fs::write(inst.game_dir.join("options.txt"), b"x").unwrap();
        repo.update(
            &inst.id,
            InstanceUpdate { memory: Some(MemoryPolicy::Fixed { min_mb: 256, max_mb: 1024 }), ..Default::default() },
        )
        .await
        .unwrap();

        let copy = repo.duplicate(&inst.id).await.unwrap();
        assert_ne!(copy.id, inst.id);
        assert_eq!(copy.name, "Original (copia)");
        assert_eq!(copy.profile_id, inst.profile_id);
        assert_eq!(copy.memory, MemoryPolicy::Fixed { min_mb: 256, max_mb: 1024 });
        assert_eq!(std::fs::read(copy.game_dir.join("mods").join("a.jar")).unwrap(), b"jar");
        assert!(copy.game_dir.join("options.txt").is_file());
        assert!(repo.get(&inst.id).await.unwrap().is_some());
        assert_eq!(repo.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn duplicate_keeps_name_within_limit() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance(&"n".repeat(64))).await.unwrap();
        let copy = repo.duplicate(&inst.id).await.unwrap();
        assert!(copy.name.chars().count() <= crate::MAX_NAME_LEN);
        assert!(copy.name.ends_with("(copia)"));
    }

    #[tokio::test]
    async fn newer_schema_is_rejected_not_overwritten() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        let path = repo.instance_dir(&inst.id).unwrap().join("instance.json");
        let raw = std::fs::read_to_string(&path).unwrap().replace("\"schemaVersion\": 1", "\"schemaVersion\": 99");
        std::fs::write(&path, raw).unwrap();

        assert!(matches!(repo.get(&inst.id).await, Err(InstanceError::UnsupportedSchema(99))));
    }
}
