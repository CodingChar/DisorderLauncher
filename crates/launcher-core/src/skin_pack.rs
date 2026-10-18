//! Skin del perfil Offline Gaming dentro del juego, sin mods.
//!
//! Minecraft en modo sin conexion no puede pedir la skin a los servidores de Mojang, asi que
//! usa una de las skins por defecto (Steve, Alex...). Un resource pack puede reemplazar esas
//! texturas: se genera `resourcepacks/DisorderLauncher-skin.zip` con la skin del perfil y se
//! activa en `options.txt`. Se rehace en cada lanzamiento; sin skin se retira.
//!
//! Limite conocido: el juego elige el modelo (brazos de 4 o 3 px) segun el UUID del perfil, no
//! segun la skin, asi que una skin de brazos finos puede verse con brazos anchos y al reves.

use std::io::{Cursor, Write};
use std::path::Path;

const PACK_FILE: &str = "DisorderLauncher-skin.zip";

/// Version 1.x -> (minor, patch). `None` para snapshots y versiones fuera de ese formato.
fn parse_version(v: &str) -> Option<(u32, u32)> {
    let mut parts = v.strip_prefix("1.")?.split('.');
    let minor = parts
        .next()?
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    let patch = parts
        .next()
        .and_then(|p| p.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok())
        .unwrap_or(0);
    Some((minor, patch))
}

/// Antes de 1.13 los resource packs se nombran sin prefijo en `options.txt`.
fn is_legacy(version: &str) -> bool {
    parse_version(version).is_some_and(|(minor, _)| minor < 13)
}

fn pack_format(version: &str) -> u32 {
    let Some((minor, patch)) = parse_version(version) else {
        return 34;
    };
    match (minor, patch) {
        (..=8, _) => 1,
        (9..=10, _) => 2,
        (11..=12, _) => 3,
        (13..=14, _) => 4,
        (15, _) | (16, 0..=1) => 5,
        (16, _) => 6,
        (17, _) => 7,
        (18, _) => 8,
        (19, 0..=2) => 9,
        (19, 3) => 12,
        (19, _) => 13,
        (20, 0..=1) => 15,
        (20, 2) => 18,
        (20, 3..=4) => 22,
        (20, _) => 32,
        (21, 0..=1) => 34,
        _ => 34,
    }
}

/// Texturas que sustituye: las clasicas (hasta 1.20.1) y las de la carpeta `player/` (desde 1.20.2).
fn texture_paths() -> Vec<String> {
    let mut paths = vec!["steve".to_string(), "alex".to_string()]
        .into_iter()
        .map(|n| format!("assets/minecraft/textures/entity/{n}.png"))
        .collect::<Vec<_>>();
    for name in [
        "steve", "alex", "ari", "efe", "kai", "makena", "noor", "sunny", "zuri",
    ] {
        for kind in ["wide", "slim"] {
            paths.push(format!(
                "assets/minecraft/textures/entity/player/{kind}/{name}.png"
            ));
        }
    }
    paths
}

fn build_pack(png: &[u8], version: &str) -> std::io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let mcmeta = format!(
        "{{\"pack\":{{\"pack_format\":{},\"description\":\"Skin de tu perfil (DisorderLauncher)\"}}}}",
        pack_format(version)
    );
    zip.start_file("pack.mcmeta", options)
        .map_err(std::io::Error::other)?;
    zip.write_all(mcmeta.as_bytes())?;
    for path in texture_paths() {
        zip.start_file(path, options)
            .map_err(std::io::Error::other)?;
        zip.write_all(png)?;
    }
    Ok(zip.finish().map_err(std::io::Error::other)?.into_inner())
}

/// Devuelve el contenido de `options.txt` con el pack activado (`enable`) o retirado.
fn update_options(current: &str, entry: &str, enable: bool) -> String {
    let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
    let idx = lines.iter().position(|l| l.starts_with("resourcePacks:"));
    let mut packs: Vec<String> = idx
        .and_then(|i| serde_json::from_str(&lines[i]["resourcePacks:".len()..]).ok())
        .unwrap_or_default();
    packs.retain(|p| p != entry);
    if enable {
        packs.push(entry.to_string()); // al final = prioridad mas alta
    }
    let line = format!(
        "resourcePacks:{}",
        serde_json::to_string(&packs).unwrap_or_else(|_| "[]".into())
    );
    match idx {
        Some(i) => lines[i] = line,
        None if enable => lines.push(line),
        None => {}
    }
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Deja la instancia con la skin del perfil (`Some(png)`) o sin pack de skin (`None`).
pub(crate) async fn sync(
    game_dir: &Path,
    minecraft_version: &str,
    png: Option<Vec<u8>>,
) -> std::io::Result<()> {
    let pack_path = game_dir.join("resourcepacks").join(PACK_FILE);
    let options_path = game_dir.join("options.txt");
    let entry = if is_legacy(minecraft_version) {
        PACK_FILE.to_string()
    } else {
        format!("file/{PACK_FILE}")
    };

    match png {
        Some(png) => {
            let bytes = build_pack(&png, minecraft_version)?;
            tokio::fs::create_dir_all(game_dir.join("resourcepacks")).await?;
            tokio::fs::write(&pack_path, bytes).await?;
            let current = tokio::fs::read_to_string(&options_path)
                .await
                .unwrap_or_default();
            let mut updated = update_options(&current, &entry, true);
            if !is_legacy(minecraft_version) && current.is_empty() {
                updated = updated.replace(
                    &format!("[\"{entry}\"]"),
                    &format!("[\"vanilla\",\"{entry}\"]"),
                );
            }
            if updated != current {
                tokio::fs::write(&options_path, updated).await?;
            }
        }
        None => {
            if pack_path.exists() {
                let _ = tokio::fs::remove_file(&pack_path).await;
                if let Ok(current) = tokio::fs::read_to_string(&options_path).await {
                    let updated = update_options(&current, &entry, false);
                    if updated != current {
                        tokio::fs::write(&options_path, updated).await?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_versions_use_plain_names_and_modern_ones_the_file_prefix() {
        assert!(is_legacy("1.12.2"));
        assert!(is_legacy("1.8"));
        assert!(!is_legacy("1.13"));
        assert!(!is_legacy("1.21.1"));
        assert!(!is_legacy("24w14a"));
        assert_eq!(pack_format("1.12.2"), 3);
        assert_eq!(pack_format("1.20.1"), 15);
        assert_eq!(pack_format("1.21.1"), 34);
    }

    #[test]
    fn options_gain_the_pack_without_touching_other_lines() {
        let out = update_options(
            "lang:es_es\nresourcePacks:[\"vanilla\"]\nfov:0.0\n",
            "file/a.zip",
            true,
        );
        assert_eq!(
            out,
            "lang:es_es\nresourcePacks:[\"vanilla\",\"file/a.zip\"]\nfov:0.0\n"
        );
        // Idempotente: no duplica.
        assert_eq!(update_options(&out, "file/a.zip", true), out);
        // Al retirarlo vuelve al estado anterior.
        assert_eq!(
            update_options(&out, "file/a.zip", false),
            "lang:es_es\nresourcePacks:[\"vanilla\"]\nfov:0.0\n"
        );
    }

    #[test]
    fn options_without_the_line_get_one_and_empty_options_stay_empty_on_removal() {
        assert_eq!(
            update_options("fov:0.0\n", "a.zip", true),
            "fov:0.0\nresourcePacks:[\"a.zip\"]\n"
        );
        assert_eq!(update_options("", "a.zip", false), "");
    }

    #[tokio::test]
    async fn pack_is_written_enabled_and_then_removed() {
        let dir = tempfile::tempdir().unwrap();
        let png = vec![1u8, 2, 3];

        sync(dir.path(), "1.12.2", Some(png.clone())).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"DisorderLauncher-skin.zip\"]\n"
        );
        let mut zip = zip::ZipArchive::new(
            std::fs::File::open(dir.path().join("resourcepacks").join(PACK_FILE)).unwrap(),
        )
        .unwrap();
        assert!(zip.by_name("pack.mcmeta").is_ok());
        let mut steve = zip
            .by_name("assets/minecraft/textures/entity/steve.png")
            .unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut steve, &mut data).unwrap();
        assert_eq!(data, png);
        drop(steve);

        sync(dir.path(), "1.12.2", None).await.unwrap();
        assert!(!dir.path().join("resourcepacks").join(PACK_FILE).exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[]\n"
        );
    }

    #[tokio::test]
    async fn modern_versions_start_with_vanilla_and_the_file_prefix() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), "1.21.1", Some(vec![0])).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\n"
        );
    }
}
