use crate::{parse_theme, Theme, ThemeEntry, ThemeError, ThemeMode};
use std::path::{Path, PathBuf};

const DEFAULT_DARK: &str = include_str!("../../../theme_examples/default-dark.theme.json");
const DEFAULT_LIGHT: &str = include_str!("../../../theme_examples/default-light.theme.json");
const SUFFIX: &str = ".theme.json";

pub const DEFAULT_DARK_ID: &str = "disorderlaucher.default.dark";
pub const DEFAULT_LIGHT_ID: &str = "disorderlaucher.default.light";

/// Temas incluidos en el binario. Se validan al iniciar: un fallo aqui seria un bug de empaquetado.
pub fn builtin_themes() -> Vec<Theme> {
    [DEFAULT_DARK, DEFAULT_LIGHT]
        .into_iter()
        .map(|json| parse_theme(json.as_bytes()).expect("los temas incluidos deben ser validos"))
        .collect()
}

pub fn default_theme(mode: ThemeMode) -> Theme {
    let id = match mode {
        ThemeMode::Dark => DEFAULT_DARK_ID,
        ThemeMode::Light => DEFAULT_LIGHT_ID,
    };
    builtin_themes().into_iter().find(|t| t.id == id).expect("tema por defecto")
}

/// Temas del usuario en `<root>/custom/<id>.theme.json`.
pub struct ThemeStore {
    custom_dir: PathBuf,
}

impl ThemeStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { custom_dir: root.as_ref().join("custom") }
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.custom_dir.join(format!("{id}{SUFFIX}"))
    }

    /// Incluidos primero y luego los del usuario. Un archivo invalido se omite (y se registra).
    pub async fn list(&self) -> Vec<ThemeEntry> {
        let mut entries: Vec<ThemeEntry> =
            builtin_themes().into_iter().map(|theme| ThemeEntry { builtin: true, theme }).collect();

        let Ok(mut dir) = tokio::fs::read_dir(&self.custom_dir).await else { return entries };
        let mut custom = Vec::new();
        while let Ok(Some(item)) = dir.next_entry().await {
            let name = item.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(SUFFIX) else { continue };
            let Ok(data) = tokio::fs::read(item.path()).await else { continue };
            match parse_theme(&data) {
                Ok(theme) if theme.id == id && !entries.iter().any(|e| e.theme.id == theme.id) => {
                    custom.push(ThemeEntry { builtin: false, theme })
                }
                Ok(_) => tracing::warn!(file = %name, "tema ignorado: id repetido o distinto del nombre"),
                Err(e) => tracing::warn!(file = %name, error = %e, "tema personalizado invalido; se ignora"),
            }
        }
        custom.sort_by_key(|e| e.theme.name.to_lowercase());
        entries.extend(custom);
        entries
    }

    /// Valida el JSON recibido y lo guarda. No sobreescribe temas existentes ni incluidos.
    pub async fn import(&self, json: &[u8]) -> Result<Theme, ThemeError> {
        let theme = parse_theme(json)?;
        if self.list().await.iter().any(|e| e.theme.id == theme.id) {
            return Err(ThemeError::Duplicate);
        }
        tokio::fs::create_dir_all(&self.custom_dir).await.map_err(|_| ThemeError::Io)?;
        let path = self.path_for(&theme.id);
        let part = path.with_extension("json.part");
        // Se guarda la forma ya validada y normalizada, no los bytes recibidos.
        let normalized = serde_json::to_vec_pretty(&theme).map_err(|_| ThemeError::Io)?;
        tokio::fs::write(&part, normalized).await.map_err(|_| ThemeError::Io)?;
        tokio::fs::rename(&part, &path).await.map_err(|_| ThemeError::Io)?;
        Ok(theme)
    }

    pub async fn remove(&self, id: &str) -> Result<(), ThemeError> {
        if builtin_themes().iter().any(|t| t.id == id) {
            return Err(ThemeError::Builtin);
        }
        // `id` llega del exterior: solo se acepta si coincide con un tema existente.
        let entries = self.list().await;
        let exists = entries.iter().any(|e| !e.builtin && e.theme.id == id);
        if !exists {
            return Err(ThemeError::NotFound);
        }
        tokio::fs::remove_file(self.path_for(id)).await.map_err(|_| ThemeError::Io)
    }

    /// Tema pedido o, si no existe o ya no es valido, el predeterminado del modo indicado.
    pub async fn resolve(&self, id: Option<&str>, fallback: ThemeMode) -> Theme {
        if let Some(id) = id {
            if let Some(entry) = self.list().await.into_iter().find(|e| e.theme.id == id) {
                return entry.theme;
            }
        }
        default_theme(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom_json(id: &str, name: &str) -> Vec<u8> {
        DEFAULT_DARK
            .replace("disorderlaucher.default.dark", id)
            .replace("Default Dark", name)
            .into_bytes()
    }

    #[tokio::test]
    async fn lists_builtin_themes_first() {
        let dir = tempfile::tempdir().unwrap();
        let list = ThemeStore::new(dir.path()).list().await;
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|e| e.builtin));
    }

    #[tokio::test]
    async fn import_list_remove_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path());

        let theme = store.import(&custom_json("mi.tema", "Mi tema")).await.unwrap();
        assert_eq!(theme.name, "Mi tema");

        let list = store.list().await;
        assert_eq!(list.len(), 3);
        assert!(!list[2].builtin);

        store.remove("mi.tema").await.unwrap();
        assert_eq!(store.list().await.len(), 2);
        assert_eq!(store.remove("mi.tema").await, Err(ThemeError::NotFound));
    }

    #[tokio::test]
    async fn builtin_and_duplicate_ids_are_protected() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path());

        assert_eq!(store.import(DEFAULT_DARK.as_bytes()).await, Err(ThemeError::Duplicate));
        assert_eq!(store.remove(DEFAULT_DARK_ID).await, Err(ThemeError::Builtin));

        store.import(&custom_json("uno", "Uno")).await.unwrap();
        assert_eq!(store.import(&custom_json("uno", "Otro")).await, Err(ThemeError::Duplicate));
    }

    #[tokio::test]
    async fn remove_cannot_escape_the_themes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path());
        std::fs::write(dir.path().join("secret.theme.json"), "x").unwrap();
        for id in ["../secret", "..\\secret", "../../x"] {
            assert_eq!(store.remove(id).await, Err(ThemeError::NotFound), "{id}");
        }
        assert!(dir.path().join("secret.theme.json").exists());
    }

    #[tokio::test]
    async fn invalid_or_tampered_files_are_skipped_and_resolve_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path());
        store.import(&custom_json("bueno", "Bueno")).await.unwrap();

        let custom = dir.path().join("custom");
        std::fs::write(custom.join("roto.theme.json"), "{ nope").unwrap();
        // El nombre del archivo no coincide con el id interno.
        std::fs::write(custom.join("otro.theme.json"), custom_json("distinto", "X")).unwrap();

        let ids: Vec<_> = store.list().await.into_iter().map(|e| e.theme.id).collect();
        assert_eq!(ids, [DEFAULT_DARK_ID, DEFAULT_LIGHT_ID, "bueno"]);

        assert_eq!(store.resolve(Some("bueno"), ThemeMode::Light).await.name, "Bueno");
        assert_eq!(store.resolve(Some("no-existe"), ThemeMode::Light).await.id, DEFAULT_LIGHT_ID);
        assert_eq!(store.resolve(None, ThemeMode::Dark).await.id, DEFAULT_DARK_ID);
    }

    #[tokio::test]
    async fn rejected_import_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path());
        let bad = DEFAULT_DARK.replace("#0C0C0C", "red").replace("disorderlaucher.default.dark", "malo");
        assert!(store.import(bad.as_bytes()).await.is_err());
        assert!(!dir.path().join("custom").join("malo.theme.json").exists());
    }
}
