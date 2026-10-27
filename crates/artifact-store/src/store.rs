use crate::hash::{hash_file, hash_reader, is_valid_hash};
use crate::ArtifactError;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::NamedTempFile;

/// Como quedo un objeto del almacen dentro de una instancia.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Hardlink,
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingested {
    pub hash: String,
    pub size: u64,
    /// `false` si el almacen ya tenia ese contenido.
    pub created: bool,
}

#[derive(Debug, Clone)]
pub struct StoredObject {
    pub hash: String,
    pub size: u64,
    pub modified: SystemTime,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    /// Abre (y crea si falta) el almacen en `root`, normalmente `<datos>/store`.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ArtifactError> {
        let store = Self { root: root.as_ref().to_path_buf() };
        fs::create_dir_all(store.objects_dir())?;
        fs::create_dir_all(store.tmp_dir())?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.root.join("sha512")
    }

    /// Descargas e importaciones en curso: en el mismo volumen que los objetos, para poder renombrar.
    pub fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    /// Ruta del objeto. Un hash mal formado se rechaza: nunca puede salir de la carpeta del almacen.
    pub fn path_of(&self, hash: &str) -> Result<PathBuf, ArtifactError> {
        if !is_valid_hash(hash) {
            return Err(ArtifactError::InvalidHash);
        }
        Ok(self.objects_dir().join(&hash[..2]).join(hash))
    }

    pub fn has(&self, hash: &str) -> bool {
        self.path_of(hash).map(|p| p.is_file()).unwrap_or(false)
    }

    pub fn size_of(&self, hash: &str) -> Result<u64, ArtifactError> {
        let path = self.path_of(hash)?;
        match fs::metadata(path) {
            Ok(meta) => Ok(meta.len()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(ArtifactError::Missing),
            Err(e) => Err(e.into()),
        }
    }

    /// Archivo temporal dentro del almacen, para escribir una descarga antes de conocer su hash.
    pub fn temp_file(&self) -> Result<NamedTempFile, ArtifactError> {
        fs::create_dir_all(self.tmp_dir())?;
        Ok(tempfile::Builder::new().prefix("obj-").suffix(".part").tempfile_in(self.tmp_dir())?)
    }

    /// Incorpora un archivo ya escrito en el almacen (p. ej. una descarga). El nombre del objeto sale
    /// del contenido real: nunca entra un objeto con nombre equivocado.
    pub fn ingest_temp(&self, temp: NamedTempFile) -> Result<Ingested, ArtifactError> {
        let (hash, size) = hash_file(temp.path())?;
        let target = self.path_of(&hash)?;
        if target.is_file() {
            return Ok(Ingested { hash, size, created: false });
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        temp.persist(&target).map_err(|e| e.error)?;
        Ok(Ingested { hash, size, created: true })
    }

    /// Incorpora un archivo existente cuyo hash ya se conoce. Prefiere un hardlink al propio archivo
    /// (sin copiar bytes); si no se puede, lo copia. Devuelve `None` si el contenido ya estaba.
    pub fn ingest_known(&self, src: &Path, hash: &str, size: u64) -> Result<Option<Method>, ArtifactError> {
        let target = self.path_of(hash)?;
        if target.is_file() {
            return Ok(None);
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::hard_link(src, &target) {
            Ok(()) => return Ok(Some(Method::Hardlink)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Ok(None),
            Err(_) => {}
        }
        let temp = self.temp_file()?;
        fs::copy(src, temp.path())?;
        if fs::metadata(temp.path())?.len() != size {
            return Err(ArtifactError::Mismatch);
        }
        temp.persist(&target).map_err(|e| e.error)?;
        Ok(Some(Method::Copy))
    }

    pub fn ingest_file(&self, src: &Path) -> Result<Ingested, ArtifactError> {
        let (hash, size) = hash_file(src)?;
        let created = self.ingest_known(src, &hash, size)?.is_some();
        Ok(Ingested { hash, size, created })
    }

    /// Deja el objeto visible en `dest` como un archivo normal: hardlink si el volumen lo permite, copia si no.
    /// Escribe a un nombre temporal y renombra, asi `dest` nunca queda a medias.
    pub fn materialize(&self, hash: &str, dest: &Path) -> Result<Method, ArtifactError> {
        let object = self.path_of(hash)?;
        if !object.is_file() {
            return Err(ArtifactError::Missing);
        }
        let parent = dest.parent().ok_or_else(|| io::Error::other("destino sin carpeta"))?;
        fs::create_dir_all(parent)?;
        let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("objeto");
        let staging = parent.join(format!(".{name}.{}.part", std::process::id()));
        let _ = fs::remove_file(&staging);
        let method = match fs::hard_link(&object, &staging) {
            Ok(()) => Method::Hardlink,
            Err(_) => {
                if let Err(e) = fs::copy(&object, &staging) {
                    let _ = fs::remove_file(&staging);
                    return Err(e.into());
                }
                Method::Copy
            }
        };
        if let Err(e) = fs::rename(&staging, dest) {
            let _ = fs::remove_file(&staging);
            return Err(e.into());
        }
        Ok(method)
    }

    /// Recalcula el hash del objeto. `false` si fue alterado (p. ej. editado en sitio por un tercero).
    pub fn verify(&self, hash: &str) -> Result<bool, ArtifactError> {
        let path = self.path_of(hash)?;
        let file = fs::File::open(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                ArtifactError::Missing
            } else {
                e.into()
            }
        })?;
        Ok(hash_reader(file)?.0 == hash)
    }

    pub fn remove(&self, hash: &str) -> Result<(), ArtifactError> {
        match fs::remove_file(self.path_of(hash)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// Todos los objetos con su tamano y fecha. Ignora lo que no tenga forma de objeto.
    pub fn list(&self) -> Result<Vec<StoredObject>, ArtifactError> {
        let mut out = Vec::new();
        for shard in fs::read_dir(self.objects_dir())? {
            let shard = shard?;
            if !shard.file_type()?.is_dir() {
                continue;
            }
            let prefix = shard.file_name().to_string_lossy().into_owned();
            for object in fs::read_dir(shard.path())? {
                let object = object?;
                let Some(hash) = object.file_name().to_str().map(str::to_owned) else { continue };
                if !is_valid_hash(&hash) || !hash.starts_with(&prefix) {
                    continue;
                }
                let meta = object.metadata()?;
                if meta.is_file() {
                    out.push(StoredObject { hash, size: meta.len(), modified: meta.modified()?, path: object.path() });
                }
            }
        }
        Ok(out)
    }
}
