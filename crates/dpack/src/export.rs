use crate::manifest::{EntrySource, LoaderInfo, Manifest, ManifestEntry, MinecraftInfo, PackInfo};
use crate::path::validate_entry_path;
use crate::{DpackError, FORMAT_VERSION, MANIFEST_NAME};
use artifact_store::{hash_file, mtime_ns, ArtifactIndex, ArtifactSource};
use sevenz_rust2::{ArchiveEntry, ArchiveWriter, EncoderConfiguration, EncoderMethod, SourceReader};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Contenido que define el pack. Mundos, capturas, logs, `options.txt` y `servers.dat` son estado
/// personal de la instancia y no viajan (los mundos solo con `include_worlds`).
const PACK_DIRS: &[&str] = &["mods", "config", "defaultconfigs", "kubejs", "scripts", "resourcepacks", "shaderpacks", "datapacks"];
const WORLD_DIR: &str = "saves";
const MAX_FILES: usize = 100_000;
/// Resource pack con la skin del perfil que genera el launcher al lanzar: es personal y se rehace sola.
const GENERATED_SKIN_PACK: &str = "DisorderLauncher-skin.zip";
/// Tamano de un bloque solido de texto: acota la memoria del compresor y los archivos abiertos.
const SOLID_BLOCK_FILES: usize = 256;
const SOLID_BLOCK_BYTES: u64 = 32 * 1024 * 1024;
/// Extensiones ya comprimidas: recomprimirlas gasta CPU sin ganar tamano, asi que van tal cual (COPY).
const ALREADY_COMPRESSED: &[&str] = &["jar", "zip", "png", "jpg", "jpeg", "ogg", "mp3", "7z", "gz", "rar", "mrpack"];

pub struct ExportRequest<'a> {
    pub name: &'a str,
    pub minecraft_version: &'a str,
    /// `vanilla`, `fabric`, `quilt`, `forge` o `neoforge`.
    pub loader_kind: &'a str,
    pub loader_version: Option<&'a str>,
    pub game_dir: &'a Path,
    pub include_worlds: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    pub files: usize,
    /// Bytes de contenido (sin comprimir).
    pub bytes: u64,
}

struct Item {
    rel: String,
    path: PathBuf,
    size: u64,
    stored: bool,
}

fn is_stored(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    let name = name.strip_suffix(".disabled").unwrap_or(&name);
    name.rsplit_once('.').is_some_and(|(_, ext)| ALREADY_COMPRESSED.contains(&ext))
}

fn walk(root: &Path, relative: &str, out: &mut Vec<Item>) -> Result<(), DpackError> {
    let Ok(entries) = fs::read_dir(root.join(relative)) else { return Ok(()) };
    for entry in entries {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = format!("{relative}/{name}");
        if kind.is_dir() {
            walk(root, &rel, out)?;
        } else if kind.is_file() {
            // `.part` son temporales a medias; un nombre fuera de la regla no se puede importar despues.
            if name.ends_with(".part") || name == GENERATED_SKIN_PACK || validate_entry_path(&rel).is_err() {
                continue;
            }
            if out.len() >= MAX_FILES {
                return Err(DpackError::TooManyEntries);
            }
            let size = entry.metadata()?.len();
            out.push(Item { stored: is_stored(&rel), rel, path: entry.path(), size });
        }
        // Los enlaces simbolicos no se siguen: no deben sacar archivos de fuera de la instancia.
    }
    Ok(())
}

fn source_of(source: &ArtifactSource) -> EntrySource {
    match source {
        ArtifactSource::Local => EntrySource::Local,
        ArtifactSource::Modrinth { project_id, version_id } => {
            EntrySource::Modrinth { project_id: project_id.clone(), version_id: version_id.clone() }
        }
    }
}

/// Lee un archivo solo cuando el compresor lo pide y lo cierra al terminar: un bloque solido no
/// mantiene cientos de archivos abiertos a la vez.
struct LazyFile {
    path: PathBuf,
    file: Option<File>,
    done: bool,
}

impl LazyFile {
    fn new(path: PathBuf) -> Self {
        Self { path, file: None, done: false }
    }
}

impl Read for LazyFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.done {
            return Ok(0);
        }
        if self.file.is_none() {
            self.file = Some(File::open(&self.path)?);
        }
        let read = self.file.as_mut().expect("abierto arriba").read(buf)?;
        if read == 0 {
            self.done = true;
            self.file = None;
        }
        Ok(read)
    }
}

fn archive_err(e: sevenz_rust2::Error) -> DpackError {
    DpackError::Archive(e.to_string())
}

/// Crea el `.7z` en `dest`. Si algo falla no queda un archivo a medias. `progress(hechos, total)`.
pub fn export(request: &ExportRequest<'_>, dest: &Path, progress: &(dyn Fn(usize, usize) + Sync)) -> Result<ExportReport, DpackError> {
    let mut items = Vec::new();
    for dir in PACK_DIRS {
        walk(request.game_dir, dir, &mut items)?;
    }
    if request.include_worlds {
        walk(request.game_dir, WORLD_DIR, &mut items)?;
    }
    items.sort_by(|a, b| a.rel.cmp(&b.rel));

    // Identidad de cada archivo: el indice de la instancia ya la tiene para lo adoptado; el resto se calcula.
    let index = ArtifactIndex::load(request.game_dir);
    let mut entries = Vec::with_capacity(items.len());
    for item in &items {
        let meta = fs::metadata(&item.path)?;
        let known = index.entries.get(&item.rel).filter(|e| e.size == meta.len() && e.mtime_ns == mtime_ns(&meta));
        let (sha512, size, source) = match known {
            Some(entry) => (entry.sha512.clone(), entry.size, source_of(&entry.source)),
            None => {
                let (hash, size) = hash_file(&item.path)?;
                (hash, size, EntrySource::Local)
            }
        };
        entries.push(ManifestEntry { path: item.rel.clone(), size, sha512, source });
    }
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        pack: PackInfo { name: request.name.to_string() },
        minecraft: MinecraftInfo { version: request.minecraft_version.to_string() },
        loader: LoaderInfo { kind: request.loader_kind.to_string(), version: request.loader_version.map(str::to_string) },
        created_by: "DisorderLauncher".to_string(),
        entries,
    };
    let bytes: u64 = manifest.entries.iter().map(|e| e.size).sum();
    let report = ExportReport { files: items.len(), bytes };

    let part = dest.with_extension("7z.part");
    let result = write_archive(&part, &manifest, &items, progress);
    match result {
        Ok(()) => fs_safe::rename_blocking(&part, dest).map_err(|e| {
            let _ = fs::remove_file(&part);
            DpackError::Io(e)
        })?,
        Err(e) => {
            let _ = fs::remove_file(&part);
            return Err(e);
        }
    }
    Ok(report)
}

fn write_archive(part: &Path, manifest: &Manifest, items: &[Item], progress: &(dyn Fn(usize, usize) + Sync)) -> Result<(), DpackError> {
    let mut writer = ArchiveWriter::create(part).map_err(archive_err)?;
    let lzma2 = || vec![EncoderConfiguration::new(EncoderMethod::LZMA2)];

    // 1. El manifiesto, primero y solo: el importador lo lee sin tocar el resto.
    writer.set_content_methods(lzma2());
    let json = serde_json::to_vec_pretty(manifest).map_err(|e| DpackError::Manifest(e.to_string()))?;
    writer
        .push_archive_entry(ArchiveEntry::new_file(MANIFEST_NAME), Some(json.as_slice()))
        .map_err(archive_err)?;

    // 2. Jars, zips e imagenes: sin recomprimir.
    let total = items.len();
    let mut done = 0usize;
    writer.set_content_methods(vec![EncoderConfiguration::new(EncoderMethod::COPY)]);
    for item in items.iter().filter(|i| i.stored) {
        writer
            .push_archive_entry(ArchiveEntry::new_file(&item.rel), Some(LazyFile::new(item.path.clone())))
            .map_err(archive_err)?;
        done += 1;
        progress(done, total);
    }

    // 3. Configuracion y texto: bloques solidos LZMA2, donde la compresion realmente rinde.
    writer.set_content_methods(lzma2());
    let mut batch: Vec<&Item> = Vec::new();
    let mut batch_bytes = 0u64;
    let flush = |writer: &mut ArchiveWriter<File>, batch: &mut Vec<&Item>, done: &mut usize| -> Result<(), DpackError> {
        if batch.is_empty() {
            return Ok(());
        }
        let entries: Vec<ArchiveEntry> = batch.iter().map(|i| ArchiveEntry::new_file(&i.rel)).collect();
        let readers: Vec<SourceReader<LazyFile>> =
            batch.iter().map(|i| SourceReader::new(LazyFile::new(i.path.clone()))).collect();
        writer.push_archive_entries(entries, readers).map_err(archive_err)?;
        *done += batch.len();
        progress(*done, total);
        batch.clear();
        Ok(())
    };
    for item in items.iter().filter(|i| !i.stored) {
        batch.push(item);
        batch_bytes += item.size;
        if batch.len() >= SOLID_BLOCK_FILES || batch_bytes >= SOLID_BLOCK_BYTES {
            flush(&mut writer, &mut batch, &mut done)?;
            batch_bytes = 0;
        }
    }
    flush(&mut writer, &mut batch, &mut done)?;

    writer.finish()?;
    Ok(())
}
