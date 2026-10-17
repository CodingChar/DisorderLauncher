#[derive(Debug, thiserror::Error)]
pub enum SkinError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("el archivo no es un PNG valido")]
    NotAPng,
    #[error("el archivo es demasiado grande para ser una skin")]
    TooLarge,
    #[error("las dimensiones {width}x{height} no son validas: usa 64x64 (o 64x32 en skins antiguas)")]
    BadDimensions { width: u32, height: u32 },
    #[error("nombre de skin invalido")]
    InvalidName,
    #[error("skin no encontrada")]
    NotFound,
    #[error("no se pudo codificar la imagen")]
    Encode,
}

// `io::Error` y `serde_json::Error` no implementan `PartialEq`: se comparan por categoria en las pruebas.
impl PartialEq for SkinError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BadDimensions { width: w1, height: h1 }, Self::BadDimensions { width: w2, height: h2 }) => w1 == w2 && h1 == h2,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}
