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
//! `pack.mcmeta` tambien cambia (ver [`PackFormat`]): hasta 1.21.8 lleva `pack_format`; desde 1.21.9 y
//! en 26.x solo `min_format`/`max_format`. Mezclar ambos hace que el juego descarte el pack y que
//! la skin no cargue. El pack tampoco debe estar en `incompatibleResourcePacks`: el juego lo saca de
//! esa lista y no lo activa en ese arranque.
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

/// Como declara un resource pack para que versiones sirve (`pack.mcmeta`).
///
/// Desde 25w31a (1.21.9) el juego exige `min_format` y `max_format` y rechaza, sin cargarlo, un pack
/// que declare `supported_formats` por encima del formato 64 sin ellos. Un pack rechazado se retira
/// de `options.txt` y el jugador ve la skin por defecto. Antes de esa version, en cambio, solo
/// existe `pack_format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackFormat {
    /// Un unico numero (hasta 1.21.8).
    Single(u32),
    /// Un rango abierto por arriba (desde 1.21.9, 26.x y las que aun no existen).
    Ranged,
}

/// Primer formato de resource pack que se declara con rango.
const RANGED_MIN_FORMAT: u32 = 65;
/// Techo del rango: el pack sirve tambien para las versiones que aun no existen.
const RANGED_MAX_FORMAT: u32 = 999;

/// Que espera el juego de una skin: donde va el pack, como se activa y que formato de textura lee.
/// Es la unica pieza que cambia al aparecer una epoca nueva del juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkinTarget {
    scheme: Scheme,
    layout: SkinLayout,
    pack_format: PackFormat,
}

/// Primera fecha (AAAA-MM-DD) de cada epoca. Solo se usa cuando el id de la version no se reconoce.
const RESOURCE_PACKS_SINCE: &str = "2013-06-13"; // snapshot 13w24a
const MODERN_LAYOUT_SINCE: &str = "2014-09-02"; // 1.8
const FILE_PREFIX_SINCE: &str = "2018-07-18"; // 1.13
const RANGED_FORMAT_SINCE: &str = "2025-07-29"; // snapshot 25w31a

/// Fecha de salida de cada version -> su formato de resource pack. Un snapshot sin formato conocido
/// toma el de la version que prepara.
const FORMAT_RELEASES: &[(&str, u32)] = &[
    ("2014-09-02", 1), // 1.8
    ("2016-02-29", 2), // 1.9
    ("2016-11-14", 3), // 1.11
    ("2018-07-18", 4), // 1.13
    ("2019-12-10", 5), // 1.15
    ("2020-08-11", 6), // 1.16.2
    ("2021-06-08", 7), // 1.17
    ("2021-11-30", 8), // 1.18
    ("2022-06-07", 9), // 1.19
    ("2022-12-07", 12), // 1.19.3
    ("2023-03-14", 13), // 1.19.4
    ("2023-06-07", 15), // 1.20
    ("2023-09-21", 18), // 1.20.2
    ("2023-12-05", 22), // 1.20.3
    ("2024-04-23", 32), // 1.20.5
    ("2024-06-13", 34), // 1.21
    ("2024-10-22", 42), // 1.21.2
    ("2024-12-03", 46), // 1.21.4
    ("2025-03-25", 55), // 1.21.5
    ("2025-06-17", 63), // 1.21.6
    ("2025-06-30", 64), // 1.21.7
];

impl SkinTarget {
    /// Epoca de una version. `ids` son los identificadores a probar en orden (la version base primero:
    /// un perfil de OptiFine o Forge se comporta como la version de Minecraft sobre la que se apoya) y
    /// `release_time` la fecha de publicacion de esa version base. Lo desconocido se trata como la
    /// version mas reciente, que es la que mejor representa a las que aun no existen.
    pub(crate) fn resolve(ids: &[&str], release_time: Option<&str>) -> Self {
        let (scheme, layout) = ids
            .iter()
            .find_map(|id| classify_id(id))
            .or_else(|| release_time.and_then(classify_date))
            .unwrap_or((Scheme::ResourcePackFile, SkinLayout::Modern));
        let pack_format = ids
            .iter()
            .find_map(|id| pack_format(id))
            .or_else(|| release_time.and_then(format_by_date))
            .unwrap_or(PackFormat::Ranged);
        Self { scheme, layout, pack_format }
    }
}

fn classify_id(v: &str) -> Option<(Scheme, SkinLayout)> {
    if is_ancient(v) {
        return Some((Scheme::TexturePack, SkinLayout::Legacy));
    }
    if let Some((minor, _)) = parse_release(v) {
        let scheme = match minor {
            ..=5 => Scheme::TexturePack,
            6..=12 => Scheme::ResourcePack,
            _ => Scheme::ResourcePackFile,
        };
        let layout = if minor < 8 { SkinLayout::Legacy } else { SkinLayout::Modern };
        return Some((scheme, layout));
    }
    let (year, week) = parse_snapshot(v)?;
    let scheme = match (year, week) {
        // 13w24a es el primer snapshot con resource packs; 17w43a, el primero con `file/`.
        (..=12, _) | (13, ..=23) => Scheme::TexturePack,
        (13..=17, _) if (year, week) < (17, 43) => Scheme::ResourcePack,
        _ => Scheme::ResourcePackFile,
    };
    let layout = if (year, week) < (14, 4) { SkinLayout::Legacy } else { SkinLayout::Modern };
    Some((scheme, layout))
}

fn classify_date(release_time: &str) -> Option<(Scheme, SkinLayout)> {
    let day = release_time.get(..10)?;
    if !day.as_bytes().iter().all(|b| b.is_ascii_digit() || *b == b'-') {
        return None;
    }
    let scheme = if day < RESOURCE_PACKS_SINCE {
        Scheme::TexturePack
    } else if day < FILE_PREFIX_SINCE {
        Scheme::ResourcePack
    } else {
        Scheme::ResourcePackFile
    };
    let layout = if day < MODERN_LAYOUT_SINCE { SkinLayout::Legacy } else { SkinLayout::Modern };
    Some((scheme, layout))
}

/// Version de la era `26.x` (el juego paso a numerarse por ano): `26.1`, `26.3.1`, `26.1-snapshot-2`...
fn is_year_release(v: &str) -> bool {
    let mut parts = v.splitn(2, '.');
    let major = parts.next().filter(|m| m.len() == 2 && m.bytes().all(|b| b.is_ascii_digit()));
    major.and_then(|m| m.parse::<u32>().ok()).is_some_and(|m| m >= 26) && parts.next().is_some()
}

/// Formato de resource pack de esa version. `None` si el id no dice la version (se prueba con la fecha).
fn pack_format(version: &str) -> Option<PackFormat> {
    if is_year_release(version) {
        return Some(PackFormat::Ranged);
    }
    if let Some((year, week)) = parse_snapshot(version) {
        // 25w31a es el primer snapshot con `min_format`/`max_format`.
        return ((year, week) >= (25, 31)).then_some(PackFormat::Ranged);
    }
    let (minor, patch) = parse_release(version)?;
    let single = match (minor, patch) {
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
        (21, 2..=3) => 42,
        (21, 4) => 46,
        (21, 5) => 55,
        (21, 6) => 63,
        (21, 7..=8) => 64,
        // 1.21.9 en adelante (y una hipotetica 1.22): formato con rango.
        _ => return Some(PackFormat::Ranged),
    };
    Some(PackFormat::Single(single))
}

/// Formato por fecha de publicacion, para ids que no se reconocen (snapshots viejos, versiones propias).
fn format_by_date(release_time: &str) -> Option<PackFormat> {
    let day = release_time.get(..10)?;
    if !day.as_bytes().iter().all(|b| b.is_ascii_digit() || *b == b'-') {
        return None;
    }
    if day >= RANGED_FORMAT_SINCE {
        return Some(PackFormat::Ranged);
    }
    let upcoming = FORMAT_RELEASES.iter().find(|(date, _)| *date > day);
    Some(PackFormat::Single(upcoming.map_or(64, |(_, f)| *f)))
}

/// Contenido de `pack.mcmeta`. Cada juego valida lo suyo y descarta el pack entero si no le cuadra,
/// asi que se declara solo lo que entiende la epoca (ver [`PackFormat`]).
fn pack_mcmeta(format: PackFormat) -> String {
    let mut pack = serde_json::Map::new();
    match format {
        PackFormat::Single(n) => {
            pack.insert("pack_format".into(), n.into());
        }
        PackFormat::Ranged => {
            pack.insert("min_format".into(), RANGED_MIN_FORMAT.into());
            pack.insert("max_format".into(), RANGED_MAX_FORMAT.into());
        }
    }
    pack.insert("description".into(), PACK_DESCRIPTION.into());
    serde_json::json!({ "pack": pack }).to_string()
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

fn build_pack(png: &[u8], target: SkinTarget) -> std::io::Result<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    match target.scheme {
        Scheme::TexturePack => {
            zip.start_file("pack.txt", options).map_err(zip_error)?;
            zip.write_all(PACK_DESCRIPTION.as_bytes())?;
            zip.start_file("mob/char.png", options).map_err(zip_error)?;
            zip.write_all(png)?;
        }
        Scheme::ResourcePack | Scheme::ResourcePackFile => {
            zip.start_file("pack.mcmeta", options).map_err(zip_error)?;
            zip.write_all(pack_mcmeta(target.pack_format).as_bytes())?;
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
    update_list_option(current, "resourcePacks:", entry, enable)
}

/// Quita el pack de `incompatibleResourcePacks`. NO debe estar ahi: si el juego encuentra en esa lista
/// un pack que ya es compatible, lo saca de la lista ("now compatible") y ese arranque no lo activa,
/// asi que la skin no carga. Una version anterior del launcher lo anotaba; se limpia.
fn forget_incompatible(current: &str, entry: &str) -> String {
    update_list_option(current, "incompatibleResourcePacks:", entry, false)
}

/// Anade o quita `entry` de una linea `<clave>["a","b"]` de `options.txt`. Las demas lineas no se tocan.
fn update_list_option(current: &str, key: &str, entry: &str, enable: bool) -> String {
    let mut lines: Vec<String> = current.lines().map(str::to_string).collect();
    let idx = lines.iter().position(|l| l.starts_with(key));
    let mut packs: Vec<String> = idx
        .and_then(|i| serde_json::from_str(&lines[i][key.len()..]).ok())
        .unwrap_or_default();
    packs.retain(|p| p != entry);
    if enable {
        packs.push(entry.to_string()); // al final = prioridad mas alta
    }
    let line = format!(
        "{key}{}",
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
    target: SkinTarget,
    skin: Option<ProfileSkin>,
) -> std::io::Result<()> {
    let scheme = target.scheme;
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
            let png = match adapt_png(&skin.png, skin.model, target.layout) {
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
                    forget_incompatible(&updated, &entry)
                }
            };
            write_atomic(&pack_path, &build_pack(&png, target)?).await?;
            write_if_changed(&options_path, &current, updated).await?;
        }
        None => {
            let _ = tokio::fs::remove_file(&pack_path).await;
            if exists {
                let updated = match scheme {
                    Scheme::TexturePack => update_texture_option(&current, &entry, false).0,
                    Scheme::ResourcePack => update_options(&current, &entry, false),
                    Scheme::ResourcePackFile => forget_incompatible(&update_options(&current, &entry, false), &entry),
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

    fn target(version: &str) -> SkinTarget {
        SkinTarget::resolve(&[version], None)
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
            let t = target(version);
            assert_eq!((t.scheme, t.layout), (expected_scheme, expected_layout), "{version}");
        }
    }

    #[test]
    fn pack_formats_follow_the_game_version() {
        use PackFormat::{Ranged, Single};
        let cases = [
            ("1.8", Single(1)),
            ("1.12.2", Single(3)),
            ("1.20.1", Single(15)),
            ("1.20.4", Single(22)),
            ("1.21.1", Single(34)),
            ("1.21.3", Single(42)),
            ("1.21.4", Single(46)),
            ("1.21.5", Single(55)),
            ("1.21.8", Single(64)),
            ("1.21.9", Ranged),
            ("1.21.11", Ranged),
            ("1.21.9-pre1", Ranged),
            ("26.1", Ranged),
            ("26.3", Ranged),
            ("26.1-snapshot-2", Ranged),
            ("25w31a", Ranged),
        ];
        for (version, expected) in cases {
            assert_eq!(target(version).pack_format, expected, "{version}");
        }
        // Snapshot semanal viejo: el formato de la version que prepara, segun su fecha.
        let by_date = |id: &str, d: &str| SkinTarget::resolve(&[id], Some(d)).pack_format;
        assert_eq!(by_date("24w10a", "2024-03-06T00:00:00+00:00"), Single(32));
        assert_eq!(by_date("25w20a", "2025-05-14T00:00:00+00:00"), Single(63));
        assert_eq!(by_date("25w45a", "2025-11-04T00:00:00+00:00"), Ranged);
        // Sin id ni fecha: la era mas reciente.
        assert_eq!(SkinTarget::resolve(&["rareza"], None).pack_format, Ranged);
    }

    /// El juego descarta el pack entero si `pack.mcmeta` no le cuadra: cada epoca declara lo suyo.
    #[test]
    fn mcmeta_declares_only_what_each_era_understands() {
        let parse = |f| serde_json::from_str::<serde_json::Value>(&pack_mcmeta(f)).unwrap()["pack"].clone();
        let old = parse(PackFormat::Single(34));
        assert_eq!(old["pack_format"], 34);
        assert!(old.get("supported_formats").is_none(), "1.21.9+ lo rechaza junto a rangos altos");
        assert!(old.get("min_format").is_none());
        let new = parse(PackFormat::Ranged);
        assert_eq!((new["min_format"].as_u64(), new["max_format"].as_u64()), (Some(65), Some(999)));
        assert!(new.get("pack_format").is_none() && new.get("supported_formats").is_none());
        assert!(!new["description"].as_str().unwrap().is_empty());
    }

    #[test]
    fn custom_profiles_follow_their_base_version_and_unknown_ids_the_release_date() {
        // OptiFine / Forge: el id propio no dice la epoca, la version base si.
        let optifine_17 = SkinTarget::resolve(&["1.7.10", "1.7.10_HD_U_E7"], Some("2014-06-26T00:00:00+00:00"));
        assert_eq!((optifine_17.scheme, optifine_17.layout), (Scheme::ResourcePack, SkinLayout::Legacy));
        let fabric_old = SkinTarget::resolve(&["1.16.5", "fabric-loader-0.15.0-1.16.5"], None);
        assert_eq!((fabric_old.scheme, fabric_old.layout), (Scheme::ResourcePackFile, SkinLayout::Modern));
        // Id irreconocible: manda la fecha.
        let by_date = |d: &str| {
            let t = SkinTarget::resolve(&["rareza"], Some(d));
            (t.scheme, t.layout)
        };
        assert_eq!(by_date("2012-01-01T00:00:00+00:00"), (Scheme::TexturePack, SkinLayout::Legacy));
        assert_eq!(by_date("2013-12-01T00:00:00+00:00"), (Scheme::ResourcePack, SkinLayout::Legacy));
        assert_eq!(by_date("2016-01-01T00:00:00+00:00"), (Scheme::ResourcePack, SkinLayout::Modern));
        assert_eq!(by_date("2020-01-01T00:00:00+00:00"), (Scheme::ResourcePackFile, SkinLayout::Modern));
        // Ni id ni fecha (versiones futuras, p. ej. 26.1): la epoca mas reciente.
        let future = SkinTarget::resolve(&["26.1"], None);
        assert_eq!((future.scheme, future.layout), (Scheme::ResourcePackFile, SkinLayout::Modern));
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
        sync(dir.path(), target("1.12.2"), profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"DisorderLauncher-skin.zip\"]\n"
        );
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        let mcmeta = String::from_utf8(read_entry(&pack, "pack.mcmeta")).unwrap();
        assert!(mcmeta.contains("\"pack_format\":3"));
        serde_json::from_str::<serde_json::Value>(&mcmeta).unwrap();
        assert_eq!(size(&read_entry(&pack, "assets/minecraft/textures/entity/steve.png")), (64, 64));
        assert_eq!(size(&read_entry(&pack, "assets/minecraft/textures/entity/alex.png")), (64, 64));

        sync(dir.path(), target("1.12.2"), None).await.unwrap();
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
        sync(dir.path(), target("1.7.3"), profile(64)).await.unwrap();
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
        sync(dir.path(), target("1.20.1"), profile(32)).await.unwrap();
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

        sync(dir.path(), target("b1.7.3"), profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "music:1.0\nskin:DisorderLauncher-skin.zip\n"
        );
        assert!(!dir.path().join("resourcepacks").join(PACK_FILE).exists());
        let pack = dir.path().join("texturepacks").join(PACK_FILE);
        assert_eq!(size(&read_entry(&pack, "mob/char.png")), (64, 32));
        assert!(!String::from_utf8(read_entry(&pack, "pack.txt")).unwrap().is_empty());

        sync(dir.path(), target("b1.7.3"), None).await.unwrap();
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
        sync(dir.path(), target("1.0"), profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "skin:Faithful.zip\n"
        );
        assert!(!dir.path().join("texturepacks").join(PACK_FILE).exists());
    }

    #[tokio::test]
    async fn modern_versions_start_with_vanilla_and_the_file_prefix() {
        let dir = tempfile::tempdir().unwrap();
        sync(dir.path(), target("1.21.1"), profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\n"
        );
    }

    /// Con el pack en `incompatibleResourcePacks` el juego lo saca de la lista y NO lo activa ese
    /// arranque (visto en 26.3): una version anterior del launcher lo dejaba ahi.
    #[tokio::test]
    async fn a_stale_incompatible_entry_is_cleaned_so_the_game_activates_the_pack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("options.txt"),
            "resourcePacks:[\"vanilla\"]\nincompatibleResourcePacks:[\"file/DisorderLauncher-skin.zip\",\"file/otro.zip\"]\n",
        )
        .unwrap();
        sync(dir.path(), target("26.3"), profile(64)).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\nincompatibleResourcePacks:[\"file/otro.zip\"]\n"
        );
        sync(dir.path(), target("26.3"), None).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\"]\nincompatibleResourcePacks:[\"file/otro.zip\"]\n"
        );
    }

    /// El caso del informe: en 1.21.9+ y 26.x el pack con `supported_formats` se descartaba.
    #[tokio::test]
    async fn recent_versions_get_a_ranged_mcmeta_and_the_new_player_textures() {
        for version in ["1.21.9", "1.21.11", "26.1", "26.3"] {
            let dir = tempfile::tempdir().unwrap();
            sync(dir.path(), target(version), profile(64)).await.unwrap();
            let pack = dir.path().join("resourcepacks").join(PACK_FILE);
            let mcmeta: serde_json::Value =
                serde_json::from_slice(&read_entry(&pack, "pack.mcmeta")).unwrap();
            assert_eq!(mcmeta["pack"]["min_format"], 65, "{version}");
            assert_eq!(mcmeta["pack"]["max_format"], 999, "{version}");
            assert!(mcmeta["pack"].get("supported_formats").is_none(), "{version}");
            for kind in ["wide", "slim"] {
                for name in ["steve", "alex", "ari", "efe", "kai", "makena", "noor", "sunny", "zuri"] {
                    let path = format!("assets/minecraft/textures/entity/player/{kind}/{name}.png");
                    assert_eq!(size(&read_entry(&pack, &path)), (64, 64), "{version} {path}");
                }
            }
        }
    }

    #[tokio::test]
    async fn removing_the_skin_cleans_options_even_if_the_zip_is_already_gone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/DisorderLauncher-skin.zip\"]\n",
        )
        .unwrap();
        sync(dir.path(), target("1.21.1"), None).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.path().join("options.txt")).unwrap(),
            "resourcePacks:[\"vanilla\"]\n"
        );
        // Sin options.txt no se crea nada.
        let empty = tempfile::tempdir().unwrap();
        sync(empty.path(), target("1.21.1"), None).await.unwrap();
        assert!(!empty.path().join("options.txt").exists());
    }

    #[tokio::test]
    async fn an_unreadable_skin_is_used_as_is_instead_of_failing_the_launch() {
        let dir = tempfile::tempdir().unwrap();
        let broken = Some(ProfileSkin { png: vec![1, 2, 3], model: SkinModel::Classic });
        sync(dir.path(), target("1.12.2"), broken).await.unwrap();
        let pack = dir.path().join("resourcepacks").join(PACK_FILE);
        assert_eq!(read_entry(&pack, "assets/minecraft/textures/entity/steve.png"), vec![1, 2, 3]);
    }
}
