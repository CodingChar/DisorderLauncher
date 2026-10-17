use crate::{check_skin_png, SkinError};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;
const MAX_NAME_LEN: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    Classic,
    Slim,
}

/// Origen de la skin. `MicrosoftProfile` queda reservado: esas skins las gestiona Microsoft y
/// no se guardan ni se editan como recursos locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkinSource {
    Local,
    MicrosoftProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinAsset {
    pub id: String,
    pub name: String,
    pub model: SkinModel,
    pub source: SkinSource,
    pub width: u32,
    pub height: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Index {
    schema_version: u32,
    skins: Vec<SkinAsset>,
}

/// `<root>/index.json` + `<root>/assets/<id>.png`. El PNG nunca va dentro de un JSON de cuenta.
pub struct SkinLibrary {
    root: PathBuf,
}

impl SkinLibrary {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn asset_path(&self, id: &str) -> Result<PathBuf, SkinError> {
        validate_id(id)?;
        Ok(self.root.join("assets").join(format!("{id}.png")))
    }

    async fn load(&self) -> Index {
        match tokio::fs::read_to_string(self.index_path()).await {
            Ok(data) => serde_json::from_str::<Index>(&data).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "index de skins ilegible; se usa una biblioteca vacia");
                Index::default()
            }),
            Err(_) => Index::default(),
        }
    }

    async fn save(&self, mut index: Index) -> Result<(), SkinError> {
        index.schema_version = SCHEMA_VERSION;
        tokio::fs::create_dir_all(&self.root).await?;
        let path = self.index_path();
        let part = path.with_extension("json.part");
        tokio::fs::write(&part, serde_json::to_string_pretty(&index)?).await?;
        tokio::fs::rename(&part, &path).await?;
        Ok(())
    }

    pub async fn list(&self) -> Vec<SkinAsset> {
        let mut skins = self.load().await.skins;
        skins.sort_by_key(|s| s.created_at);
        skins
    }

    pub async fn get(&self, id: &str) -> Option<SkinAsset> {
        self.load().await.skins.into_iter().find(|s| s.id == id)
    }

    /// Valida y guarda una skin. `model` fuerza classic/slim; si es `None` se detecta.
    pub async fn import(&self, bytes: &[u8], name: &str, model: Option<SkinModel>) -> Result<SkinAsset, SkinError> {
        let checked = check_skin_png(bytes)?;
        let name = validate_name(name)?;

        let id = format!("skin_{}", uuid::Uuid::new_v4().simple());
        let path = self.asset_path(&id)?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let part = path.with_extension("png.part");
        tokio::fs::write(&part, &checked.png).await?;
        tokio::fs::rename(&part, &path).await?;

        let now = Utc::now();
        let asset = SkinAsset {
            id,
            name,
            model: model.unwrap_or(checked.detected_model),
            source: SkinSource::Local,
            width: checked.width,
            height: checked.height,
            created_at: now,
            updated_at: now,
        };
        let mut index = self.load().await;
        index.skins.push(asset.clone());
        self.save(index).await?;
        Ok(asset)
    }

    /// Reemplaza los pixeles de una skin existente (editor). Conserva id y nombre.
    pub async fn replace_pixels(&self, id: &str, bytes: &[u8]) -> Result<SkinAsset, SkinError> {
        let checked = check_skin_png(bytes)?;
        let mut index = self.load().await;
        let entry = index.skins.iter_mut().find(|s| s.id == id).ok_or(SkinError::NotFound)?;

        let path = self.asset_path(id)?;
        let part = path.with_extension("png.part");
        tokio::fs::write(&part, &checked.png).await?;
        tokio::fs::rename(&part, &path).await?;

        entry.width = checked.width;
        entry.height = checked.height;
        entry.updated_at = Utc::now();
        let updated = entry.clone();
        self.save(index).await?;
        Ok(updated)
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<SkinAsset, SkinError> {
        let name = validate_name(name)?;
        self.update(id, |s| s.name = name).await
    }

    pub async fn set_model(&self, id: &str, model: SkinModel) -> Result<SkinAsset, SkinError> {
        self.update(id, |s| s.model = model).await
    }

    async fn update(&self, id: &str, change: impl FnOnce(&mut SkinAsset)) -> Result<SkinAsset, SkinError> {
        let mut index = self.load().await;
        let entry = index.skins.iter_mut().find(|s| s.id == id).ok_or(SkinError::NotFound)?;
        change(entry);
        entry.updated_at = Utc::now();
        let updated = entry.clone();
        self.save(index).await?;
        Ok(updated)
    }

    pub async fn remove(&self, id: &str) -> Result<(), SkinError> {
        let mut index = self.load().await;
        let before = index.skins.len();
        index.skins.retain(|s| s.id != id);
        if index.skins.len() == before {
            return Err(SkinError::NotFound);
        }
        self.save(index).await?;
        let _ = tokio::fs::remove_file(self.asset_path(id)?).await;
        Ok(())
    }

    /// Bytes del PNG normalizado.
    pub async fn read_png(&self, id: &str) -> Result<Vec<u8>, SkinError> {
        self.get(id).await.ok_or(SkinError::NotFound)?;
        Ok(tokio::fs::read(self.asset_path(id)?).await?)
    }

    /// El PNG como data URL, listo para `<img>` o `<canvas>` en la interfaz.
    pub async fn read_data_url(&self, id: &str) -> Result<String, SkinError> {
        Ok(format!("data:image/png;base64,{}", STANDARD.encode(self.read_png(id).await?)))
    }

    /// Ruta del PNG en disco (para mostrarla a mods compatibles o exportarla).
    pub async fn asset_file(&self, id: &str) -> Result<PathBuf, SkinError> {
        self.get(id).await.ok_or(SkinError::NotFound)?;
        self.asset_path(id)
    }
}

/// `skin_` + 32 hex: nunca contiene separadores de ruta.
fn validate_id(id: &str) -> Result<(), SkinError> {
    let ok = id.strip_prefix("skin_").is_some_and(|rest| rest.len() == 32 && rest.chars().all(|c| c.is_ascii_hexdigit()));
    if ok { Ok(()) } else { Err(SkinError::NotFound) }
}

fn validate_name(name: &str) -> Result<String, SkinError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LEN || name.chars().any(char::is_control) {
        return Err(SkinError::InvalidName);
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode_rgba;

    fn png(slim: bool) -> Vec<u8> {
        let mut rgba = vec![255u8; 64 * 64 * 4];
        if slim {
            for (x, y) in [(47u32, 20u32), (47, 24), (39, 52), (39, 56)] {
                rgba[((y * 64 + x) * 4 + 3) as usize] = 0;
            }
        }
        encode_rgba(&rgba, 64, 64).unwrap()
    }

    #[tokio::test]
    async fn import_list_rename_model_remove() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(dir.path());
        assert!(lib.list().await.is_empty());

        let classic = lib.import(&png(false), "  Mi skin  ", None).await.unwrap();
        assert_eq!(classic.name, "Mi skin");
        assert_eq!(classic.model, SkinModel::Classic);
        assert_eq!(classic.source, SkinSource::Local);
        let slim = lib.import(&png(true), "Alex", None).await.unwrap();
        assert_eq!(slim.model, SkinModel::Slim);
        assert!(dir.path().join("assets").join(format!("{}.png", classic.id)).is_file());

        let forced = lib.import(&png(false), "Forzada", Some(SkinModel::Slim)).await.unwrap();
        assert_eq!(forced.model, SkinModel::Slim);

        assert_eq!(lib.list().await.len(), 3);
        assert_eq!(lib.rename(&classic.id, "Nueva").await.unwrap().name, "Nueva");
        assert_eq!(lib.set_model(&classic.id, SkinModel::Slim).await.unwrap().model, SkinModel::Slim);

        lib.remove(&classic.id).await.unwrap();
        assert!(lib.get(&classic.id).await.is_none());
        assert!(!dir.path().join("assets").join(format!("{}.png", classic.id)).exists());
        assert_eq!(lib.remove(&classic.id).await.unwrap_err(), SkinError::NotFound);
    }

    #[tokio::test]
    async fn invalid_input_is_rejected_and_nothing_is_stored() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(dir.path());
        assert_eq!(lib.import(b"not a png", "x", None).await.unwrap_err(), SkinError::NotAPng);
        assert_eq!(lib.import(&png(false), "  ", None).await.unwrap_err(), SkinError::InvalidName);
        assert_eq!(lib.import(&png(false), &"n".repeat(49), None).await.unwrap_err(), SkinError::InvalidName);
        assert_eq!(lib.import(&png(false), "a\nb", None).await.unwrap_err(), SkinError::InvalidName);
        assert!(lib.list().await.is_empty());
        let assets = std::fs::read_dir(dir.path().join("assets")).map(|d| d.count()).unwrap_or(0);
        assert_eq!(assets, 0);
    }

    #[tokio::test]
    async fn ids_cannot_escape_the_assets_directory() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(dir.path());
        std::fs::write(dir.path().join("secret.png"), "x").unwrap();
        for id in ["../secret", "skin_../../x", "", "skin_short", "..\\x"] {
            assert_eq!(lib.read_png(id).await.unwrap_err(), SkinError::NotFound, "{id}");
            assert_eq!(lib.remove(id).await.unwrap_err(), SkinError::NotFound, "{id}");
        }
        assert!(dir.path().join("secret.png").exists());
    }

    #[tokio::test]
    async fn read_data_url_and_replace_pixels() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(dir.path());
        let asset = lib.import(&png(false), "Base", None).await.unwrap();

        let url = lib.read_data_url(&asset.id).await.unwrap();
        assert!(url.starts_with("data:image/png;base64,"));

        let edited = lib.replace_pixels(&asset.id, &png(true)).await.unwrap();
        assert_eq!(edited.id, asset.id);
        assert_eq!(edited.name, "Base");
        assert!(lib.replace_pixels(&asset.id, b"nope").await.is_err());
        assert_eq!(lib.replace_pixels("skin_00000000000000000000000000000000", &png(false)).await.unwrap_err(), SkinError::NotFound);
    }

    #[tokio::test]
    async fn corrupt_index_falls_back_to_empty_library() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.json"), "{ nope").unwrap();
        let lib = SkinLibrary::new(dir.path());
        assert!(lib.list().await.is_empty());
        assert!(lib.import(&png(false), "Ok", None).await.is_ok());
    }
}
