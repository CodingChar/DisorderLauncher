//! Skin del perfil Offline Gaming dentro del juego, sin mods.
//!
//! Minecraft en modo sin conexion no puede pedir la skin a los servidores de Mojang, asi que
//! usa una de las skins por defecto (Steve, Alex...). Un pack de texturas puede reemplazar esas
//! imagenes: se genera `DisorderLauncher-skin.zip` con la skin del perfil y se activa en
//! `options.txt`. Se rehace en cada lanzamiento; sin skin se retira.
//!
//! Cada epoca del juego lo espera distinto (ver [`Scheme`] y [`SkinLayout`]):
//!
//! | Version          | Carpeta         | `options.txt`                       | Textura             |
//! |------------------|-----------------|-------------------------------------|---------------------|
//! | Alpha/Beta - 1.5 | `texturepacks/` | `skin:<archivo>`                    | `mob/char.png` 64x32 |
//! | 1.6 - 1.7        | `resourcepacks/`| `resourcePacks:["<archivo>"]`       | `steve.png` 64x32    |
//! | 1.8 - 1.12       | `resourcepacks/`| `resourcePacks:["<archivo>"]`       | `steve/alex` 64x64   |
//! | 1.13+            | `resourcepacks/`| `resourcePacks:["file/<archivo>"]`  | `steve/alex` 64x64   |
//!
//! Con el formato equivocado el juego lee otras filas de la imagen y la skin sale cortada y con
//! partes vacias, por eso la imagen se adapta a la version antes de empaquetarla.
//!
//! Limite conocido: el juego elige el modelo (brazos de 4 o 3 px) segun el UUID del perfil, no
//! segun la skin, asi que una skin de brazos finos puede verse con brazos anchos y al reves.
//! Antes de 1.8 solo existe el modelo de brazos anchos.

use skins::{adapt_png, SkinLayout, SkinModel};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

const PACK_FILE: &str = "DisorderLauncher-skin.zip";
const PACK_DESCRIPTION: &str = "Skin de tu perfil (DisorderLauncher)";

/// La skin del perfil y el modelo de brazos con el que se dibujo.
pub(crate) struct ProfileSkin {
    pub png: Vec<u8>,
    pub model: SkinModel,
}

/// Como carga el juego los packs de texturas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    /// `texturepacks/` y `skin:` en `options.txt` (hasta 1.5.2, incluidas alpha y beta).
    TexturePack,
    /// `resourcepacks/` con el nombre del archivo a secas (1.6 a 1.12).
    ResourcePack,
    /// `resourcepacks/` con el prefijo `file/` (desde 1.13).
    ResourcePackFile,
}

/// Version -> (menor, parche) para `1.x`. `None` si no tiene ese formato.
fn parse_release(v: &str) -> Option<(u32, u32)> {
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

/// Snapshot semanal `AAwSSx` (p. ej. `13w49a`) -> (ano, semana).
fn parse_snapshot(v: &str) -> Option<(u32, u32)> {
    let b = v.as_bytes();
    let letter = *b.last()?;
    let ok = b.len() == 6 && b[2] == b'w' && letter.is_ascii_lowercase();
    if !ok || !b[..2].iter().chain(&b[3..5]).all(u8::is_ascii_digit) {
        return None;
    }
    Some((v[..2].parse().ok()?, v[3..5].parse().ok()?))
}

/// Versiones previas al sistema de packs de texturas actual: Classic, Alpha y Beta.
fn is_ancient(v: &str) -> bool {
    ["rd-", "c0.", "inf-", "in-", "a1.", "b1."].iter().any(|p| v.starts_with(p))
}

fn scheme(version: &str) -> Scheme {
    if is_ancient(version) {
        return Scheme::TexturePack;
    }
    if let Some((minor, _)) = parse_release(version) {
        return match minor {
            ..=5 => Scheme::TexturePack,
            6..=12 => Scheme::ResourcePack,
            _ => Scheme::ResourcePackFile,
        };
    }
    match parse_snapshot(version) {
        // 13w24a es el primer snapshot con resource packs; 17w43a, el primero con `file/`.
        Some((year, week)) if (year, week) < (13, 24) => Scheme::TexturePack,
        Some((year, week)) if (year, week) < (17, 43) => Scheme::ResourcePack,
        _ => Scheme::ResourcePackFile,
    }
}

/// Textura que espera el modelo del jugador: 64x32 hasta 1.7 y 64x64 desde 1.8.
fn layout(version: &str) -> SkinLayout {
    if is_ancient(version) {
        return SkinLayout::Legacy;
    }
    if let Some((minor, _)) = parse_release(version) {
        return if minor < 8 { SkinLayout::Legacy } else { SkinLayout::Modern };
    }
    match parse_snapshot(version) {
        Some((year, _)) if year <= 13 => SkinLayout::Legacy,
        _ => SkinLayout::Modern,
    }
}

fn pack_format(version: &str) -> u32 {
    let Some((minor, patch)) = parse_release(version) else {
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

/// Texturas que sustituye en un resource pack: las clasicas (hasta 1.20.1) y las de la carpeta
/// `player/` (desde 1.20.2).
fn resource_pack_textures() -> Vec<String> {
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

fn zip_error(e: zip::result::ZipError) -> std::io::Error {
    std::io::Error::other(e)
}

fn build_pack(png: &[u8], version: &str, scheme: Scheme) -> std::io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    match scheme {
        Scheme::TexturePack => {
            zip.start_file("pack.txt", options).map_err(zip_error)?;
            zip.write_all(PACK_DESCRIPTION.as_bytes())?;
            zip.start_file("mob/char.png", options).map_err(zip_error)?;
            zip.write_all(png)?;
        }
        Scheme::ResourcePack | Scheme::ResourcePackFile => {
            // `supported_formats` (1.20.2+) evita el aviso de «pack para otra version» en las
            // versiones nuevas; las antiguas ignoran la clave.
            let mcmeta = format!(
                "{{\"pack\":{{\"pack_format\":{},\"supported_formats\":{{\"min_inclusive\":1,\"max_inclusive\":999}},\"description\":\"{PACK_DESCRIPTION}\"}}}}",
                pack_format(version)
            );
            zip.start_file("pack.mcmeta", options).map_err(zip_error)?;
            zip.write_all(mcmeta.as_bytes())?;
            for path in resource_pack_textures() {
                zip.start_file(path, options).map_err(zip_error)?;
                zip.write_all(png)?;
            }
        }
    }
    Ok(zip.finish().map_err(zip_error)?.into_inner())
}

/// Devuelve el contenido de `options.txt` con el pack activado (`enable`) o retirado de
/// `resourcePacks`. Las demas lineas no se tocan.
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
    join_lines(lines)
}

/// Antes de 1.6 solo hay un pack activo (`skin:<archivo>`). Se activa el nuestro salvo que el
/// jugador haya elegido otro a mano; en ese caso no se pisa su eleccion. Devuelve el texto nuevo
/// y si el pack quedo activo.
fn update_texture_option(current: &str, entry: &str, enable: bool) -> (String, bool) {
    let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
    let idx = lines.iter().position(|l| l.starts_with("skin:"));
    let chosen = idx.map(|i| lines[i]["skin:".len()..].trim().to_string()).unwrap_or_default();
    let ours = chosen == entry;
    let free = chosen.is_empty() || chosen.eq_ignore_ascii_case("default") || ours;
    if enable {
        if !free {
            return (current.to_string(), false);
        }
        let line = format!("skin:{entry}");
        match idx {
            Some(i) => lines[i] = line,
            None => lines.push(line),
        }
        return (join_lines(lines), true);
    }
    if ours {
        if let Some(i) = idx {
            lines[i] = "skin:Default".to_string();
        }
        return (join_lines(lines), false);
    }
    (current.to_string(), false)
}

fn join_lines(lines: Vec<String>) -> String {
    let mut out = lines.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Escribe `bytes` en `path` sin dejar nunca un archivo a medias (si el juego lo lee a la vez o el
/// equipo se apaga a mitad, queda el anterior o el nuevo).
async fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let part = path.with_extension("zip.part");
    tokio::fs::write(&part, bytes).await?;
    tokio::fs::rename(&part, path).await
}

async fn write_if_changed(path: &Path, current: &str, updated: String) -> std::io::Result<()> {
    if updated != current {
        tokio::fs::write(path, updated).await?;
    }
    Ok(())
}

/// Carpeta del pack de cada esquema.
fn pack_dir(game_dir: &Path, scheme: Scheme) -> PathBuf {
    match scheme {
        Scheme::TexturePack => game_dir.join("texturepacks"),
        Scheme::ResourcePack | Scheme::ResourcePackFile => game_dir.join("resourcepacks"),
    }
}

/// Retira restos de otras epocas: versiones anteriores del launcher dejaban el pack en
/// `resourcepacks/` aunque el juego buscara en `texturepacks/`.
async fn remove_stale(game_dir: &Path, keep: Scheme) {
    for other in [Scheme::TexturePack, Scheme::ResourcePack] {
        let same_dir = pack_dir(game_dir, other) == pack_dir(game_dir, keep);
        if !same_dir {
            let _ = tokio::fs::remove_file(pack_dir(game_dir, other).join(PACK_FILE)).await;
        }
    }
}

/// Deja la instancia con la skin del perfil (`Some`) o sin pack de skin (`None`).
pub(crate) async fn sync(
    game_dir: &Path,
    minecraft_version: &str,
    skin: Option<ProfileSkin>,
) -> std::io::Result<()> {
    let scheme = scheme(minecraft_version);
    let pack_path = pack_dir(game_dir, scheme).join(PACK_FILE);
    let options_path = game_dir.join("options.txt");
    let entry = match scheme {
        Scheme::ResourcePackFile => format!("file/{PACK_FILE}"),
        Scheme::TexturePack | Scheme::ResourcePack => PACK_FILE.to_string(),
    };
    remove_stale(game_dir, scheme).await;
    let current = tokio::fs::read_to_string(&options_path).await;
    let exists = current.is_ok();
    let current = current.unwrap_or_default();

    match skin {
        Some(skin) => {
            let png = match adapt_png(&skin.png, skin.model, layout(minecraft_version)) {
                Ok(png) => png,
                Err(e) => {
                    // La biblioteca ya valido el PNG al guardarlo; si falla aqui se prefiere la
                    // skin sin adaptar a dejar al jugador sin ella.
                    tracing::warn!(error = %e, "no se pudo adaptar la skin a esta version; se usa tal cual");
                    skin.png
                }
            };
            let updated = match scheme {
                Scheme::TexturePack => {
                    let (updated, applied) = update_texture_option(&current, &entry, true);
                    if !applied {
                        tracing::warn!("el jugador eligio otro pack de texturas; la skin del perfil no se aplica");
                        return Ok(());
                    }
                    updated
                }
                Scheme::ResourcePack | Scheme::ResourcePackFile => {
                    let mut updated = update_options(&current, &entry, true);
                    if scheme == Scheme::ResourcePackFile && current.is_empty() {
                        updated = updated.replace(
                            &format!("[\"{entry}\"]"),
                            &format!("[\"vanilla\",\"{entry}\"]"),
                        );
                    }
                    updated
                }
            };
            write_atomic(&pack_path, &build_pack(&png, minecraft_version, scheme)?).await?;
            write_if_changed(&options_path, &current, updated).await?;
        }
        None => {
            let _ = tokio::fs::remove_file(&pack_path).await;
            if exists {
                let updated = match scheme {
                    Scheme::TexturePack => update_texture_option(&current, &entry, false).0,
                    Scheme::ResourcePack | Scheme::ResourcePackFile => update_options(&current, &entry, false),
                };
                write_if_changed(&options_path, &current, updated).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// PNG 64x64 donde cada pixel codifica su fila en el canal verde.
    fn skin(height: u32) -> Vec<u8> {
        let mut rgba = vec![0u8; (64 * height * 4) as usize];
        for y in 0..height {
            for x in 0..64u32 {
                let i = ((y * 64 + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&[x as u8, y as u8, 9, 255]);
            }
        }
        skins::encode_rgba(&rgba, 64, height).unwrap()
    }

    fn profile(height: u32) -> Option<ProfileSkin> {
        Some(ProfileSkin { png: skin(height), model: SkinModel::Classic })
    }

    fn read_entry(zip_path: &Path, name: &str) -> Vec<u8> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(zip_path).unwrap()).unwrap();
        let mut data = Vec::new();
        zip.by_name(name).unwrap().read_to_end(&mut data).unwrap();
        data
    }

    fn size(png: &[u8]) -> (u32, u32) {
        let c = skins::check_skin_png(png).unwrap();
        (c.width, c.height)
    }

    #[test]
    fn each_era_gets_its_scheme_and_layout() {
        let cases = [
            ("b1.7.3", Scheme::TexturePack, SkinLayout::Legacy),
            ("a1.2.6", Scheme::TexturePack, SkinLayout::Legacy),
            ("rd-132211", Scheme::TexturePack, SkinLayout::Legacy),
            ("1.0", Scheme::TexturePack, SkinLayout::Legacy),
            ("1.5.2", Scheme::TexturePack, SkinLayout::Legacy),
            ("1.6.4", Scheme::ResourcePack, SkinLayout::Legacy),
            ("1.7.3", Scheme::ResourcePack, SkinLayout::Legacy),
            ("1.7.10", Scheme::ResourcePack, SkinLayout::Legacy),
            ("1.7.10-pre4", Scheme::ResourcePack, SkinLayout::Legacy),
            ("1.8", Scheme::ResourcePack, SkinLayout::Modern),
            ("1.12.2", Scheme::ResourcePack, SkinLayout::Modern),
            ("1.13", Scheme::ResourcePackFile, SkinLayout::Modern),
            ("1.21.1", Scheme::ResourcePackFile, SkinLayout::Modern),
            ("12w30e", Scheme::TexturePack, SkinLayout::Legacy),
            ("13w49a", Scheme::ResourcePack, SkinLayout::Legacy),
            ("15w14a", Scheme::ResourcePack, SkinLayout::Modern),
            ("17w43a", Scheme::ResourcePackFile, SkinLayout::Modern),
            ("24w14a", Scheme::ResourcePackFile, SkinLayout::Modern),
        ];
        for (version, expected_scheme, expected_layout) in cases {
            assert_eq!(scheme(version), expected_scheme, "{version}");
            assert_eq!(layout(version), expected_layout, "{version}");
        }
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

    #[test]
    fn texture_pack_option_respects_a_pack_the_player_chose() {
        let (on, applied) = update_texture_option("fov:0.0\nskin:Default\n", "a.zip", true);
        assert!(applied);
        assert_eq!(on, "fov:0.0\nskin:a.zip\n");
        assert_eq!(update_texture_option(&on, "a.zip", true), (on.clone(), true));
        assert_eq!(update_texture_option(&on, "a.zip", false).0, "fov:0.0\nskin:Default\n");

        let (ours, applied) = update_texture_option("fov:0.0\n", "a.zip", true);
        assert!(applied);
        assert_eq!(ours, "fov:0.0\nskin:a.zip\n");

        let custom = "skin:Faithful.zip\nfov:0.0\n";
        assert_eq!(update_texture_option(custom, "a.zip", true), (custom.to_string(), false));
        assert_eq!(update_texture_option(custom, "a.zip", false).0, custom, "no se toca lo ajeno");
    }

    #[tokio::test]
    async fn resource_pack_is_written_enabled_and_then_removed() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), "1.12.2", profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"DisorderLauncher-skin.zip\"]\n"
        );
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        let mcmeta = String::from_utf8(read_entry(&pack, "pack.mcmeta")).unwrap();
        assert!(mcmeta.contains("\"pack_format\":3"));
        assert!(mcmeta.contains("supported_formats"));
        serde_json::from_str::<serde_json::Value>(&mcmeta).unwrap();
        assert_eq!(size(&read_entry(&pack, "assets/minecraft/textures/entity/steve.png")), (64, 64));
        assert_eq!(size(&read_entry(&pack, "assets/minecraft/textures/entity/alex.png")), (64, 64));

        sync(dir.path(), "1.12.2", None).await.unwrap();
        assert!(!pack.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[]\n"
        );
    }

    /// El caso del informe: en 1.7.x una skin 64x64 salia cortada y con partes vacias.
    #[tokio::test]
    async fn pre_1_8_gets_a_64x32_texture() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), "1.7.3", profile(64)).await.unwrap();
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        let steve = read_entry(&pack, "assets/minecraft/textures/entity/steve.png");
        assert_eq!(size(&steve), (64, 32));
        // Es la mitad superior de la skin: la fila 20 sigue en la fila 20.
        let rgba = skins::check_skin_png(&steve).unwrap().rgba;
        assert_eq!(&rgba[(20 * 64 + 5) * 4..(20 * 64 + 5) * 4 + 3], &[5, 20, 9]);
    }

    #[tokio::test]
    async fn a_legacy_skin_is_upgraded_for_1_8_and_later() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), "1.20.1", profile(32)).await.unwrap();
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        let steve = read_entry(&pack, "assets/minecraft/textures/entity/player/wide/steve.png");
        assert_eq!(size(&steve), (64, 64));
    }

    #[tokio::test]
    async fn old_versions_use_texturepacks_and_the_skin_option() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("options.txt"), "music:1.0\nskin:Default\n").unwrap();
        // Resto de una version anterior del launcher: estaba en la carpeta equivocada.
        std::fs::create_dir_all(dir.path().join("resourcepacks")).unwrap();
        std::fs::write(dir.path().join("resourcepacks").join(PACK_FILE), "viejo").unwrap();

        sync(dir.path(), "b1.7.3", profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "music:1.0\nskin:DisorderLauncher-skin.zip\n"
        );
        assert!(!dir.path().join("resourcepacks").join(PACK_FILE).exists());
        let pack = dir.path().join("texturepacks").join(PACK_FILE);
        assert_eq!(size(&read_entry(&pack, "mob/char.png")), (64, 32));
        assert!(!String::from_utf8(read_entry(&pack, "pack.txt")).unwrap().is_empty());

        sync(dir.path(), "b1.7.3", None).await.unwrap();
        assert!(!pack.exists());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "music:1.0\nskin:Default\n"
        );
    }

    #[tokio::test]
    async fn a_texture_pack_chosen_by_the_player_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("options.txt"), "skin:Faithful.zip\n").unwrap();
        sync(dir.path(), "1.0", profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "skin:Faithful.zip\n"
        );
        assert!(!dir.path().join("texturepacks").join(PACK_FILE).exists());
    }

    #[tokio::test]
    async fn modern_versions_start_with_vanilla_and_the_file_prefix() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), "1.21.1", profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\n"
        );
    }

    #[tokio::test]
    async fn removing_the_skin_cleans_options_even_if_the_zip_is_already_gone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\n",
        )
        .unwrap();
        sync(dir.path(), "1.21.1", None).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\"]\n"
        );
        // Sin options.txt no se crea nada.
        let empty = tempfile::tempdir().unwrap();
        sync(empty.path(), "1.21.1", None).await.unwrap();
        assert!(!empty.path().join("options.txt").exists());
    }

    #[tokio::test]
    async fn an_unreadable_skin_is_used_as_is_instead_of_failing_the_launch() {
        let dir = tempfile::tempdir().unwrap();
        let broken = Some(ProfileSkin { png: vec![1, 2, 3], model: SkinModel::Classic });
        sync(dir.path(), "1.12.2", broken).await.unwrap();
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        assert_eq!(read_entry(&pack, "assets/minecraft/textures/entity/steve.png"), vec![1, 2, 3]);
    }
}
