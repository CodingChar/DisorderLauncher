//! Assets de versiones antiguas: el cliente los busca por nombre, no por hash.
//! `virtual` -> `assets/virtual/<indice>/<nombre>`; `map_to_resources` -> `<gameDir>/resources/<nombre>`.

use crate::InstallError;
use minecraft_meta::AssetIndex;
use std::path::{Component, Path, PathBuf};

/// Directorios destino segun el indice (vacio para indices modernos).
fn targets(index: &AssetIndex, index_id: &str, data_dir: &Path, game_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if index.is_virtual {
        dirs.push(data_dir.join("assets").join("virtual").join(index_id));
    }
    if index.map_to_resources {
        if let Some(game_dir) = game_dir {
            dirs.push(game_dir.join("resources"));
        }
    }
    dirs
}

/// Solo rutas relativas formadas por componentes normales (sin `..`, raiz ni unidad).
fn safe_relative(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if name.is_empty() || !path.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(path.to_path_buf())
}

fn object_path(data_dir: &Path, hash: &str) -> Option<PathBuf> {
    if !minecraft_meta::is_valid_asset_hash(hash) {
        return None;
    }
    let prefix = hash.get(..2)?;
    Some(data_dir.join("assets").join("objects").join(prefix).join(hash))
}

/// `true` si los destinos legacy requeridos ya contienen todos los assets.
pub fn legacy_assets_present(
    index: &AssetIndex,
    index_id: &str,
    data_dir: &Path,
    game_dir: Option<&Path>,
) -> bool {
    let dirs = targets(index, index_id, data_dir, game_dir);
    index.objects.iter().all(|(name, obj)| {
        let Some(rel) = safe_relative(name) else { return true };
        dirs.iter().all(|dir| {
            std::fs::metadata(dir.join(&rel)).map(|m| m.is_file() && m.len() == obj.size).unwrap_or(false)
        })
    })
}

/// Copia los assets descargados a los destinos legacy. Idempotente: omite lo que ya esta.
pub async fn materialize_legacy_assets(
    index: AssetIndex,
    index_id: String,
    data_dir: PathBuf,
    game_dir: Option<PathBuf>,
) -> Result<(), InstallError> {
    tokio::task::spawn_blocking(move || -> Result<(), InstallError> {
        let dirs = targets(&index, &index_id, &data_dir, game_dir.as_deref());
        if dirs.is_empty() {
            return Ok(());
        }
        for (name, obj) in &index.objects {
            let Some(rel) = safe_relative(name) else {
                tracing::warn!(asset = %name, "nombre de asset inseguro, se omite");
                continue;
            };
            let Some(source) = object_path(&data_dir, &obj.hash) else { continue };
            for dir in &dirs {
                let dest = dir.join(&rel);
                let up_to_date = std::fs::metadata(&dest).map(|m| m.len() == obj.size).unwrap_or(false);
                if up_to_date {
                    continue;
                }
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&source, &dest)?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| InstallError::Io(std::io::Error::other(e.to_string())))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_meta::AssetObject;
    use std::collections::HashMap;

    fn index(virtual_: bool, resources: bool, names: &[(&str, &str, u64)]) -> AssetIndex {
        AssetIndex {
            objects: names
                .iter()
                .map(|(n, h, s)| (n.to_string(), AssetObject { hash: h.to_string(), size: *s }))
                .collect::<HashMap<_, _>>(),
            is_virtual: virtual_,
            map_to_resources: resources,
        }
    }

    fn store(data: &Path, hash: &str, bytes: &[u8]) {
        let p = data.join("assets/objects").join(&hash[..2]).join(hash);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    #[tokio::test]
    async fn virtual_index_is_mapped_by_name() {
        let dir = tempfile::tempdir().unwrap();
        store(dir.path(), "abcdef0123456789abcdef0123456789abcdef01", b"icon");
        let idx = index(true, false, &[("icons/icon_16x16.png", "abcdef0123456789abcdef0123456789abcdef01", 4)]);

        assert!(!legacy_assets_present(&idx, "legacy", dir.path(), None));
        materialize_legacy_assets(idx.clone(), "legacy".into(), dir.path().into(), None).await.unwrap();

        let dest = dir.path().join("assets/virtual/legacy/icons/icon_16x16.png");
        assert_eq!(std::fs::read(dest).unwrap(), b"icon");
        assert!(legacy_assets_present(&idx, "legacy", dir.path(), None));
    }

    #[tokio::test]
    async fn map_to_resources_uses_the_game_dir() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        store(dir.path(), "abcdef0123456789abcdef0123456789abcdef01", b"snd");
        let idx = index(false, true, &[("sound/a.ogg", "abcdef0123456789abcdef0123456789abcdef01", 3)]);

        materialize_legacy_assets(idx, "pre-1.6".into(), dir.path().into(), Some(game.clone())).await.unwrap();
        assert!(game.join("resources/sound/a.ogg").is_file());
    }

    #[tokio::test]
    async fn modern_index_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let idx = index(false, false, &[("minecraft/lang/es.json", "abcdef0123456789abcdef0123456789abcdef01", 3)]);
        materialize_legacy_assets(idx.clone(), "17".into(), dir.path().into(), None).await.unwrap();
        assert!(!dir.path().join("assets/virtual").exists());
        assert!(legacy_assets_present(&idx, "17", dir.path(), None));
    }

    #[tokio::test]
    async fn unsafe_names_are_ignored_and_never_escape() {
        let dir = tempfile::tempdir().unwrap();
        store(dir.path(), "abcdef0123456789abcdef0123456789abcdef01", b"x");
        let idx = index(true, false, &[("../../escape.txt", "abcdef0123456789abcdef0123456789abcdef01", 1), ("/abs.txt", "abcdef0123456789abcdef0123456789abcdef01", 1)]);

        materialize_legacy_assets(idx, "legacy".into(), dir.path().into(), None).await.unwrap();
        assert!(!dir.path().join("escape.txt").exists());
        assert!(!dir.path().join("assets/escape.txt").exists());
    }

    #[test]
    fn safe_relative_rules() {
        assert!(safe_relative("a/b.png").is_some());
        assert!(safe_relative("../a").is_none());
        assert!(safe_relative("").is_none());
    }
}
