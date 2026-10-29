use crate::ArtifactError;
use sha2::{Digest, Sha512};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const BUFFER: usize = 256 * 1024;

/// SHA-512 en hexadecimal minuscula (128 caracteres).
pub fn is_valid_hash(hash: &str) -> bool {
    hash.len() == 128 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Hash en streaming: nunca carga el archivo completo en memoria.
pub fn hash_reader(mut reader: impl Read) -> Result<(String, u64), ArtifactError> {
    let mut hasher = Sha512::new();
    let mut buffer = vec![0u8; BUFFER];
    let mut total = 0u64;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total += read as u64;
    }
    Ok((to_hex(&hasher.finalize()), total))
}

pub fn hash_file(path: &Path) -> Result<(String, u64), ArtifactError> {
    hash_reader(File::open(path)?)
}

fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 15) as usize] as char);
    }
    out
}

/// Hash incremental, para calcular el SHA-512 mientras se escribe un archivo.
pub struct Sha512Hasher(Sha512);

impl Sha512Hasher {
    pub fn new() -> Self {
        Self(Sha512::new())
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finalize_hex(self) -> String {
        to_hex(&self.0.finalize())
    }
}

impl Default for Sha512Hasher {
    fn default() -> Self {
        Self::new()
    }
}
