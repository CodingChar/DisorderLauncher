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
    /// Serializa los cambios al indice: dos guardados a la vez (p. ej. doble clic en «Guardar») leian el
    /// mismo indice y el ultimo en escribir borraba la skin del otro.
    write_lock: tokio::sync::Mutex<()>,
}

impl SkinLibrary {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf(), write_lock: tokio::sync::Mutex::new(()) }
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn asset_path(&self, id: &str) -> Result<PathBuf, SkinError> {
        validate_id(id)?;
        Ok(self.root.join("assets").join(format!("{id}.png")))
    }

    async fn load(&self) -> Index {
        let data = match tokio::fs::read_to_string(self.index_path()).await {
            Ok(data) => data,
            Err(_) => return Index::default(),
        };
        match serde_json::from_str::<Index>(&data) {
            Ok(index) => index,
            Err(e) => {
                tracing::warn!(error = %e, "index de skins ilegible; se aparta y se reconstruye con los PNG guardados");
                self.recover().await
            }
        }
    }

    /// Aparta el indice danado (`index.json.corrupt`) y lo reconstruye con los PNG de `assets/`:
    /// se pierden los nombres, no las skins.
    async fn recover(&self) -> Index {
        let _ = tokio::fs::rename(self.index_path(), self.root.join("index.json.corrupt")).await;
        let mut index = Index { schema_version: SCHEMA_VERSION, skins: Vec::new() };
        if let Ok(mut dir) = tokio::fs::read_dir(self.root.join("assets")).await {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();
                let Some(id) = name.strip_suffix(".png") else { continue };
                if validate_id(id).is_err() {
                    continue;
                }
                let Ok(bytes) = tokio::fs::read(entry.path()).await else { continue };
                let Ok(checked) = check_skin_png(&bytes) else { continue };
                let when = entry
                    .metadata()
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(DateTime::<Utc>::from)
                    .unwrap_or_else(Utc::now);
                index.skins.push(SkinAsset {
                    id: id.to_string(),
                    name: format!("Skin recuperada {}", index.skins.len() + 1),
                    model: checked.detected_model,
                    source: SkinSource::Local,
                    width: checked.width,
                    height: checked.height,
                    created_at: when,
                    updated_at: when,
                });
            }
        }
        index.skins.sort_by_key(|s| s.created_at);
        if let Err(e) = self.save(Index { schema_version: SCHEMA_VERSION, skins: index.skins.clone() }).await {
            tracing::warn!(error = %e, "no se pudo guardar el indice reconstruido");
        }
        index
    }

    async fn save(&self, mut index: Index) -> Result<(), SkinError> {
        index.schema_version = SCHEMA_VERSION;
        tokio::fs::create_dir_all(&self.root).await?;
        let path = self.index_path();
        fs_safe::write_atomic(&path, serde_json::to_string_pretty(&index)?).await?;
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
        fs_safe::write_atomic(&path, &checked.png).await?;

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
        let _guard = self.write_lock.lock().await;
        let mut index = self.load().await;
        index.skins.push(asset.clone());
        self.save(index).await?;
        Ok(asset)
    }

    /// Reemplaza los pixeles de una skin existente (editor). Conserva id y nombre.
    pub async fn replace_pixels(&self, id: &str, bytes: &[u8]) -> Result<SkinAsset, SkinError> {
        self.replace(id, bytes, None).await
    }

    /// Valida el documento completo antes de escribir y serializa sus pixeles y metadatos juntos.
    pub async fn save_edited(&self, id: &str, bytes: &[u8], name: &str, model: SkinModel) -> Result<SkinAsset, SkinError> {
        let name = validate_name(name)?;
        self.replace(id, bytes, Some((name, model))).await
    }

    async fn replace(&self, id: &str, bytes: &[u8], metadata: Option<(String, SkinModel)>) -> Result<SkinAsset, SkinError> {
        let checked = check_skin_png(bytes)?;
        let _guard = self.write_lock.lock().await;
        let mut index = self.load().await;
        let entry = index.skins.iter_mut().find(|s| s.id == id).ok_or(SkinError::NotFound)?;

        let path = self.asset_path(id)?;
        fs_safe::write_atomic(&path, &checked.png).await?;

        entry.width = checked.width;
        entry.height = checked.height;
        if let Some((name, model)) = metadata {
            entry.name = name;
            entry.model = model;
        }
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
        let _guard = self.write_lock.lock().await;
        let mut index = self.load().await;
        let entry = index.skins.iter_mut().find(|s| s.id == id).ok_or(SkinError::NotFound)?;
        change(entry);
        entry.updated_at = Utc::now();
        let updated = entry.clone();
        self.save(index).await?;
        Ok(updated)
    }

    pub async fn remove(&self, id: &str) -> Result<(), SkinError> {
        let _guard = self.write_lock.lock().await;
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn simultaneous_imports_never_lose_a_skin() {
        let dir = tempfile::tempdir().unwrap();
        let lib = std::sync::Arc::new(SkinLibrary::new(dir.path()));
        let bytes = png(false);
        let tasks: Vec<_> = (0..20)
            .map(|i| {
                let lib = lib.clone();
                let bytes = bytes.clone();
                tokio::spawn(async move { lib.import(&bytes, &format!("Skin {i}"), None).await.unwrap() })
            })
            .collect();
        for t in tasks {
            t.await.unwrap();
        }
        assert_eq!(lib.list().await.len(), 20);
    }

    #[tokio::test]
    async fn a_corrupt_index_is_set_aside_and_rebuilt_from_the_saved_pngs() {
        let dir = tempfile::tempdir().unwrap();
        let lib = SkinLibrary::new(dir.path());
        let a = lib.import(&png(false), "Una", None).await.unwrap();
        let b = lib.import(&png(true), "Otra", None).await.unwrap();

        std::fs::write(dir.path().join("index.json"), "basura {").unwrap();
        let skins = lib.list().await;
        assert_eq!(skins.len(), 2, "no se pierde ninguna skin");
        assert!(skins.iter().any(|s| s.id == a.id) && skins.iter().any(|s| s.id == b.id));
        assert!(skins.iter().all(|s| s.name.starts_with("Skin recuperada")));
        assert_eq!(skins.iter().find(|s| s.id == b.id).unwrap().model, SkinModel::Slim);
        assert!(dir.path().join("index.json.corrupt").is_file());
        // Ya reconstruido: se puede leer y seguir usando con normalidad.
        assert_eq!(lib.list().await.len(), 2);
        assert!(lib.read_png(&a.id).await.is_ok());
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn editor_saves_keep_pixels_and_metadata_from_the_same_document() {
        let dir = tempfile::tempdir().unwrap();
        let lib = std::sync::Arc::new(SkinLibrary::new(dir.path()));
        let asset = lib.import(&png(false), "Original", None).await.unwrap();
        let mut tasks = Vec::new();
        for i in 0..16u8 {
            let lib = lib.clone();
            let id = asset.id.clone();
            tasks.push(tokio::spawn(async move {
                let rgba = vec![i; 64 * 64 * 4];
                let bytes = encode_rgba(&rgba, 64, 64).unwrap();
                let model = if i.is_multiple_of(2) { SkinModel::Classic } else { SkinModel::Slim };
                lib.save_edited(&id, &bytes, &format!("Documento {i}"), model).await.unwrap();
            }));
        }
        for task in tasks { task.await.unwrap(); }
        let reopened = SkinLibrary::new(dir.path());
        let saved = reopened.get(&asset.id).await.unwrap();
        let pixels = check_skin_png(&reopened.read_png(&asset.id).await.unwrap()).unwrap().rgba;
        let value = pixels[0];
        assert!(pixels.iter().all(|&p| p == value));
        assert_eq!(saved.name, format!("Documento {value}"));
        assert_eq!(saved.model, if value.is_multiple_of(2) { SkinModel::Classic } else { SkinModel::Slim });
        let before = reopened.read_png(&asset.id).await.unwrap();
        assert!(reopened.save_edited(&asset.id, b"corrupto", "Nuevo", SkinModel::Classic).await.is_err());
        assert_eq!(reopened.read_png(&asset.id).await.unwrap(), before);
        assert_eq!(reopened.get(&asset.id).await.unwrap(), saved);
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
