use crate::{Instance, InstanceError, JavaSelection, LoaderKind, MemoryPolicy};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const CURRENT_SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceFile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderFile,
    pub profile_id: String,
    pub java: JavaSelection,
    pub memory: MemoryPolicy,
    /// Campo agregado despues: los archivos antiguos no lo tienen.
    #[serde(default)]
    pub icon: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct LoaderFile {
    pub kind: LoaderKind,
    #[serde(default)]
    pub version: Option<String>,
}

impl InstanceFile {
    pub fn from_instance(instance: &Instance) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA,
            id: instance.id.clone(),
            name: instance.name.clone(),
            minecraft_version: instance.minecraft_version.clone(),
            loader: LoaderFile {
                kind: instance.loader,
                version: instance.loader_version.clone(),
            },
            profile_id: instance.profile_id.clone(),
            java: instance.java.clone(),
            memory: instance.memory.clone(),
            icon: instance.icon.clone(),
            created_at: instance.created_at,
            updated_at: instance.updated_at,
        }
    }

    pub fn into_instance(self, game_dir: PathBuf) -> Result<Instance, InstanceError> {
        if self.schema_version > CURRENT_SCHEMA {
            return Err(InstanceError::UnsupportedSchema(self.schema_version));
        }
        Ok(Instance {
            id: self.id,
            name: self.name,
            minecraft_version: self.minecraft_version,
            loader: self.loader.kind,
            loader_version: self.loader.version,
            profile_id: self.profile_id,
            java: self.java,
            memory: self.memory,
            icon: self.icon,
            game_dir,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
