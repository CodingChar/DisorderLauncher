use std::path::{Path, PathBuf};

/// Estructura de directorios del launcher bajo una unica raiz de datos.
#[derive(Debug, Clone)]
pub struct LauncherPaths {
    root: PathBuf,
}

impl LauncherPaths {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn accounts_dir(&self) -> PathBuf {
        self.root.join("accounts")
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.root.join("instances")
    }

    /// Raiz compartida de Minecraft: `versions/`, `libraries/`, `assets/`.
    pub fn minecraft_dir(&self) -> PathBuf {
        self.root.join("minecraft")
    }

    pub fn runtimes_dir(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn manifest_cache(&self) -> PathBuf {
        self.root.join("cache").join("version_manifest.json")
    }

    pub fn version_descriptor(&self, version_id: &str) -> PathBuf {
        self.minecraft_dir()
            .join("versions")
            .join(version_id)
            .join(format!("{version_id}.json"))
    }
}
