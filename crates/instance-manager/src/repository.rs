use crate::file::InstanceFile;
use crate::model::{validate_icon, validate_jvm_args, validate_jvm_tweaks, validate_memory, validate_name, validate_required, validate_version};
use crate::{
    Instance, InstanceError, InstanceUpdate, JavaSelection, MemoryPolicy, NewInstance, Optimization,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use tokio::fs;

const ID_PREFIX: &str = "inst_";
const BACKUP_FILE: &str = "instance.json.bak";

/// Carpeta dentro de `instances/` con forma de instancia pero sin una ficha utilizable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderProblem {
    pub id: String,
    pub kind: FolderProblemKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderProblemKind {
    /// No hay `instance.json`: resto de un borrado incompleto.
    Orphan,
    /// Existe pero no se puede leer ni recuperar.
    Unreadable(String),
}
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
    mutations: Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>,
}

impl FileInstanceRepository {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf(), mutations: Mutex::new(HashMap::new()) }
    }

    async fn mutation_guard(&self, id: &str) -> Result<tokio::sync::OwnedMutexGuard<()>, InstanceError> {
        validate_id(id)?;
        let lock = {
            let mut locks = self.mutations.lock().unwrap_or_else(|p| p.into_inner());
            locks.retain(|_, lock| lock.strong_count() > 0);
            match locks.get(id).and_then(Weak::upgrade) {
                Some(lock) => lock,
                None => {
                    let lock = Arc::new(tokio::sync::Mutex::new(()));
                    locks.insert(id.to_string(), Arc::downgrade(&lock));
                    lock
                }
            }
        };
        Ok(lock.lock_owned().await)
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
        match self.read_file(id, &dir, &path).await {
            Ok(found) => Ok(found),
            // El archivo esta ilegible pero queda la copia del ultimo guardado bueno: se restaura. Un formato
            // mas nuevo o un id distinto no se "arreglan" con la copia: se informan tal cual.
            Err(primary @ InstanceError::Serialization(_)) => {
                // Un JSON valido que no encaja (variante o campo de una version mas nueva) no es un dano: no se pisa.
                let broken = match fs::read_to_string(&path).await {
                    Ok(text) => serde_json::from_str::<serde_json::Value>(&text).is_err(),
                    Err(_) => false,
                };
                if !broken {
                    return Err(primary);
                }
                let backup = dir.join(BACKUP_FILE);
                match self.read_file(id, &dir, &backup).await {
                    Ok(Some(instance)) => {
                        tracing::warn!(instance = %id, error = %primary, "instance.json danado; se restauro la copia");
                        let _ = fs::copy(&backup, &path).await;
                        Ok(Some(instance))
                    }
                    _ => Err(primary),
                }
            }
            Err(other) => Err(other),
        }
    }

    async fn read_file(&self, id: &str, dir: &Path, path: &Path) -> Result<Option<Instance>, InstanceError> {
        let data = match fs::read_to_string(path).await {
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
        // Copia del guardado anterior (si era valido) para poder recuperarse de un archivo danado.
        if self.read_file(&instance.id, &dir, &path).await.is_ok_and(|found| found.is_some()) {
            let _ = fs::copy(&path, dir.join(BACKUP_FILE)).await;
        }
        fs::rename(&part, &path).await?;
        Ok(())
    }

    /// Recrea `game/` y sus subcarpetas si se borraron desde fuera (no toca lo que ya existe).
    /// Devuelve `true` si hubo que crear algo.
    pub async fn ensure_layout(&self, id: &str) -> Result<bool, InstanceError> {
        let dir = self.instance_dir(id)?;
        let game = dir.join("game");
        let missing = !dir.join("logs").is_dir() || GAME_SUBDIRS.iter().any(|sub| !game.join(sub).is_dir());
        if missing {
            self.create_layout(id).await?;
        }
        Ok(missing)
    }

    /// Revisa la carpeta de instancias: carpetas con forma de instancia que no se pueden leer.
    pub async fn scan(&self) -> Result<Vec<FolderProblem>, InstanceError> {
        let mut entries = match fs::read_dir(&self.root).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut problems = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if validate_id(&name).is_err() || !entry.file_type().await.is_ok_and(|t| t.is_dir()) {
                continue;
            }
            match self.read(&name).await {
                Ok(Some(_)) => {}
                // Sin instance.json: sobra de un borrado incompleto (p. ej. con el juego abierto).
                Ok(None) => problems.push(FolderProblem { id: name, kind: FolderProblemKind::Orphan }),
                Err(e) => problems.push(FolderProblem { id: name, kind: FolderProblemKind::Unreadable(e.to_string()) }),
            }
        }
        problems.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(problems)
    }

    /// Elimina una carpeta huerfana (sin `instance.json` legible ni copia). No toca instancias validas.
    pub async fn remove_orphan(&self, id: &str) -> Result<bool, InstanceError> {
        let _guard = self.mutation_guard(id).await?;
        if self.read(id).await.ok().flatten().is_some() {
            return Ok(false);
        }
        match fs::remove_dir_all(self.instance_dir(id)?).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
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
        // El id queda reservado hasta escribir instance.json: la limpieza de huerfanas espera en vez de borrarlo.
        let _reserved = self.mutation_guard(&id).await?;
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
            jvm_tweaks: Vec::new(),
            jvm_args: String::new(),
            game_dir,
            created_at: now,
            updated_at: now,
        };
        self.write(&instance).await?;
        Ok(instance)
    }

    async fn update(&self, id: &str, changes: InstanceUpdate) -> Result<Instance, InstanceError> {
        let _guard = self.mutation_guard(id).await?;
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
        if let Some(tweaks) = changes.jvm_tweaks {
            instance.jvm_tweaks = validate_jvm_tweaks(&tweaks)?;
        }
        if let Some(args) = changes.jvm_args {
            instance.jvm_args = validate_jvm_args(&args)?;
        }
        if let Some(icon) = changes.icon {
            instance.icon = validate_icon(&icon)?;
        }
        instance.updated_at = Utc::now();
        self.write(&instance).await?;
        Ok(instance)
    }

    async fn delete(&self, id: &str) -> Result<(), InstanceError> {
        let _guard = self.mutation_guard(id).await?;
        let dir = self.instance_dir(id)?;
        // La ficha se quita primero: si algun archivo queda bloqueado (antivirus, juego abierto) la instancia ya
        // no aparece y la carpeta sobrante se limpia como huerfana en el siguiente arranque.
        for file in ["instance.json", BACKUP_FILE, "instance.json.part"] {
            let _ = fs::remove_file(dir.join(file)).await;
        }
        let mut attempt = 0;
        loop {
            match fs::remove_dir_all(&dir).await {
                Ok(()) => return Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(_) if attempt < 2 => {
                    attempt += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    async fn duplicate(&self, id: &str) -> Result<Instance, InstanceError> {
        let _guard = self.mutation_guard(id).await?;
        let source = self
            .read(id)
            .await?
            .ok_or_else(|| InstanceError::NotFound(id.to_string()))?;

        let new_id = format!("{ID_PREFIX}{}", uuid::Uuid::new_v4().simple());
        let _reserved = self.mutation_guard(&new_id).await?;
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
    async fn jvm_options_are_validated_normalized_and_persisted() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        assert!(inst.jvm_tweaks.is_empty() && inst.jvm_args.is_empty());

        let updated = repo
            .update(
                &inst.id,
                InstanceUpdate {
                    jvm_tweaks: Some(vec!["g1".into(), "no-explicit-gc".into(), "g1".into()]),
                    jvm_args: Some("  -XX:MaxGCPauseMillis=100    -Dfoo=bar ".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.jvm_tweaks, ["g1", "no-explicit-gc"]);
        assert_eq!(updated.jvm_args, "-XX:MaxGCPauseMillis=100 -Dfoo=bar");
        assert_eq!(repo.get(&inst.id).await.unwrap().unwrap().jvm_args, updated.jvm_args);

        for bad in ["-javaagent:x.jar", "-XX:OnError=calc", "-Xmx9G", "sin-guion"] {
            let err = repo.update(&inst.id, InstanceUpdate { jvm_args: Some(bad.into()), ..Default::default() }).await;
            assert!(matches!(err, Err(InstanceError::Invalid(_))), "debia rechazar {bad}");
        }
        let err = repo.update(&inst.id, InstanceUpdate { jvm_tweaks: Some(vec!["inventado".into()]), ..Default::default() }).await;
        assert!(matches!(err, Err(InstanceError::Invalid(_))));
        // Un rechazo no pisa lo guardado.
        assert_eq!(repo.get(&inst.id).await.unwrap().unwrap().jvm_args, updated.jvm_args);
    }

    #[tokio::test]
    async fn old_instance_files_without_jvm_options_still_load() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("Vieja")).await.unwrap();
        let path = repo.instance_dir(&inst.id).unwrap().join("instance.json");
        let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("jvmTweaks");
        json.as_object_mut().unwrap().remove("jvmArgs");
        std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();
        let loaded = repo.get(&inst.id).await.unwrap().unwrap();
        assert!(loaded.jvm_tweaks.is_empty() && loaded.jvm_args.is_empty());
    }

    #[tokio::test]
    async fn damaged_instance_json_is_restored_from_the_backup() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("Original")).await.unwrap();
        // Segundo guardado: deja la copia del primero.
        repo.update(&inst.id, InstanceUpdate { name: Some("Renombrada".into()), ..Default::default() }).await.unwrap();
        let path = repo.instance_dir(&inst.id).unwrap().join("instance.json");
        std::fs::write(&path, "{ esto no es json").unwrap();

        let restored = repo.get(&inst.id).await.unwrap().expect("se recupera de la copia");
        assert_eq!(restored.name, "Original", "vuelve al ultimo guardado valido");
        assert!(serde_json::from_str::<serde_json::Value>(&std::fs::read_to_string(&path).unwrap()).is_ok());
    }

    #[tokio::test]
    async fn newer_schema_is_reported_not_overwritten_by_the_backup() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        repo.update(&inst.id, InstanceUpdate { name: Some("B".into()), ..Default::default() }).await.unwrap();
        let path = repo.instance_dir(&inst.id).unwrap().join("instance.json");
        let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["schemaVersion"] = 99.into();
        std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();
        assert!(matches!(repo.get(&inst.id).await, Err(InstanceError::UnsupportedSchema(99))));
    }

    #[tokio::test]
    async fn valid_json_with_unknown_shape_is_not_replaced_by_the_backup() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        repo.update(&inst.id, InstanceUpdate { name: Some("B".into()), ..Default::default() }).await.unwrap();
        let path = repo.instance_dir(&inst.id).unwrap().join("instance.json");
        let mut json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        json["loader"]["kind"] = "loader-del-futuro".into();
        let written = serde_json::to_string(&json).unwrap();
        std::fs::write(&path, &written).unwrap();

        assert!(repo.get(&inst.id).await.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), written, "el archivo de la version nueva queda intacto");
    }

    #[tokio::test]
    async fn an_instance_being_created_is_never_taken_for_an_orphan() {
        let (_d, repo) = repo();
        let id = "inst_000000000000000000000000000000aa";
        let guard = repo.mutation_guard(id).await.unwrap();
        std::fs::create_dir_all(repo.instance_dir(id).unwrap().join("game")).unwrap();
        let repo = std::sync::Arc::new(repo);
        let cleaner = {
            let repo = repo.clone();
            tokio::spawn(async move { repo.remove_orphan(id).await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!cleaner.is_finished(), "la limpieza espera mientras el id esta reservado");
        drop(guard);
        assert!(cleaner.await.unwrap().unwrap(), "sin reserva ni ficha si es huerfana");
    }

    #[tokio::test]
    async fn ensure_layout_recreates_folders_deleted_from_outside() {
        let (_d, repo) = repo();
        let inst = repo.create(new_instance("A")).await.unwrap();
        assert!(!repo.ensure_layout(&inst.id).await.unwrap(), "intacta: nada que hacer");

        std::fs::remove_dir_all(&inst.game_dir).unwrap();
        std::fs::remove_dir_all(repo.logs_dir(&inst.id).unwrap()).unwrap();
        assert!(repo.ensure_layout(&inst.id).await.unwrap());
        assert!(inst.game_dir.join("mods").is_dir() && repo.logs_dir(&inst.id).unwrap().is_dir());
    }

    #[tokio::test]
    async fn scan_finds_orphans_and_delete_leaves_no_trace() {
        let (_d, repo) = repo();
        let keep = repo.create(new_instance("Sana")).await.unwrap();
        let gone = repo.create(new_instance("Rota")).await.unwrap();
        // Borrado externo parcial: queda la carpeta sin ficha.
        std::fs::remove_file(repo.instance_dir(&gone.id).unwrap().join("instance.json")).unwrap();

        let problems = repo.scan().await.unwrap();
        assert_eq!(problems, vec![FolderProblem { id: gone.id.clone(), kind: FolderProblemKind::Orphan }]);
        assert!(!repo.remove_orphan(&keep.id).await.unwrap(), "una instancia valida nunca se borra como huerfana");
        assert!(repo.remove_orphan(&gone.id).await.unwrap());
        assert!(repo.scan().await.unwrap().is_empty());

        repo.delete(&keep.id).await.unwrap();
        assert!(!repo.instance_dir(&keep.id).unwrap().exists());
        assert!(repo.list().await.unwrap().is_empty());
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
    async fn concurrent_updates_preserve_both_changes() {
        let (_d, repo) = repo();
        let instance = repo.create(new_instance("A")).await.unwrap();
        let (name, memory) = tokio::join!(
            repo.update(&instance.id, InstanceUpdate { name: Some("B".into()), ..Default::default() }),
            repo.update(&instance.id, InstanceUpdate {
                memory: Some(MemoryPolicy::Fixed { min_mb: 512, max_mb: 2048 }),
                ..Default::default()
            }),
        );
        name.unwrap();
        memory.unwrap();
        let loaded = repo.get(&instance.id).await.unwrap().unwrap();
        assert_eq!(loaded.name, "B");
        assert_eq!(loaded.memory, MemoryPolicy::Fixed { min_mb: 512, max_mb: 2048 });
    }

    #[tokio::test]
    async fn mutation_of_another_instance_does_not_wait_for_a_busy_instance() {
        let (_d, repo) = repo();
        let first = repo.create(new_instance("A")).await.unwrap();
        let second = repo.create(new_instance("B")).await.unwrap();
        let _busy = repo.mutation_guard(&first.id).await.unwrap();
        let updated = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            repo.update(&second.id, InstanceUpdate { name: Some("C".into()), ..Default::default() }),
        ).await.expect("unrelated instances must not block each other").unwrap();
        assert_eq!(updated.name, "C");
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
