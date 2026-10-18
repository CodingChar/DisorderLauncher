use crate::{SkinError, SkinModel};
use std::io::Cursor;

/// Una skin valida pesa pocos KB; mas que esto no es una skin.
pub const MAX_SKIN_BYTES: usize = 256 * 1024;
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Dimensiones permitidas: formato moderno 64x64 y formato antiguo 64x32.
pub const ALLOWED_SIZES: [(u32, u32); 2] = [(64, 64), (64, 32)];

/// Skin ya validada y normalizada.
#[derive(Debug, Clone)]
pub struct CheckedSkin {
    pub width: u32,
    pub height: u32,
    /// PNG recodificado en RGBA de 8 bits, sin metadatos.
    pub png: Vec<u8>,
    /// Los mismos pixeles ya decodificados (`width * height * 4` bytes RGBA).
    pub rgba: Vec<u8>,
    pub detected_model: SkinModel,
}

impl CheckedSkin {
    pub fn is_legacy(&self) -> bool {
        self.height == 32
    }
}

/// Valida que `bytes` sea realmente un PNG (firma, no solo extension), decodificable, de tamano
/// permitido y con dimensiones validas. Devuelve una version normalizada: RGBA8 sin chunks extra.
pub fn check_skin_png(bytes: &[u8]) -> Result<CheckedSkin, SkinError> {
    if bytes.len() > MAX_SKIN_BYTES {
        return Err(SkinError::TooLarge);
    }
    if bytes.len() < PNG_SIGNATURE.len() || bytes[..8] != PNG_SIGNATURE {
        return Err(SkinError::NotAPng);
    }

    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    decoder.set_limits(png::Limits { bytes: 1024 * 1024 });
    let mut reader = decoder.read_info().map_err(|_| SkinError::NotAPng)?;

    // Se comprueban las dimensiones antes de reservar el buffer de pixeles.
    let (width, height) = {
        let info = reader.info();
        (info.width, info.height)
    };
    if !ALLOWED_SIZES.contains(&(width, height)) {
        return Err(SkinError::BadDimensions { width, height });
    }

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).map_err(|_| SkinError::NotAPng)?;
    let rgba = to_rgba(&buf[..frame.buffer_size()], frame.color_type)?;
    if rgba.len() != (width * height * 4) as usize {
        return Err(SkinError::NotAPng);
    }

    let detected_model = detect_model(&rgba, width, height);
    let png = encode_rgba(&rgba, width, height)?;
    Ok(CheckedSkin { width, height, png, rgba, detected_model })
}

fn to_rgba(data: &[u8], color: png::ColorType) -> Result<Vec<u8>, SkinError> {
    Ok(match color {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => data.as_chunks::<2>().0.iter().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        // `normalize_to_color8` expande las paletas; no deberia llegar aqui.
        png::ColorType::Indexed => return Err(SkinError::NotAPng),
    })
}

pub fn encode_rgba(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, SkinError> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| SkinError::Encode)?;
        writer.write_image_data(rgba).map_err(|_| SkinError::Encode)?;
    }
    Ok(out)
}

/// Heuristica: en skins slim (Alex) la columna extra del brazo queda transparente.
/// Las skins antiguas 64x32 siempre son classic.
pub fn detect_model(rgba: &[u8], width: u32, height: u32) -> SkinModel {
    if height < 64 {
        return SkinModel::Classic;
    }
    let alpha = |x: u32, y: u32| rgba[((y * width + x) * 4 + 3) as usize];
    // Brazo derecho (u=40..55, v=16..31) y brazo izquierdo (u=32..47, v=48..63): cuarta columna del frente.
    let slim = alpha(47, 20) == 0 && alpha(47, 24) == 0 && alpha(39, 52) == 0 && alpha(39, 56) == 0;
    if slim { SkinModel::Slim } else { SkinModel::Classic }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG RGBA 64x64 solido; `transparent` borra la columna de brazo slim.
    pub(crate) fn skin_png(width: u32, height: u32, slim: bool) -> Vec<u8> {
        let mut rgba = vec![0u8; (width * height * 4) as usize];
        for px in rgba.as_chunks_mut::<4>().0 {
            *px = [120, 80, 60, 255];
        }
        if slim && height == 64 {
            for (x, y) in [(47u32, 20u32), (47, 24), (39, 52), (39, 56)] {
                rgba[((y * width + x) * 4 + 3) as usize] = 0;
            }
        }
        encode_rgba(&rgba, width, height).unwrap()
    }

    #[test]
    fn accepts_modern_and_legacy_sizes_and_normalizes() {
        let modern = check_skin_png(&skin_png(64, 64, false)).unwrap();
        assert_eq!((modern.width, modern.height), (64, 64));
        assert!(!modern.is_legacy());
        let legacy = check_skin_png(&skin_png(64, 32, false)).unwrap();
        assert!(legacy.is_legacy());
        assert_eq!(legacy.detected_model, SkinModel::Classic);
        assert_eq!(&modern.png[..8], &PNG_SIGNATURE);
    }

    #[test]
    fn detects_slim_and_classic() {
        assert_eq!(check_skin_png(&skin_png(64, 64, true)).unwrap().detected_model, SkinModel::Slim);
        assert_eq!(check_skin_png(&skin_png(64, 64, false)).unwrap().detected_model, SkinModel::Classic);
    }

    #[test]
    fn rejects_non_png_even_if_named_png() {
        assert_eq!(check_skin_png(b"GIF89a....").unwrap_err(), SkinError::NotAPng);
        assert_eq!(check_skin_png(b"").unwrap_err(), SkinError::NotAPng);
        assert_eq!(check_skin_png(b"<svg onload=alert(1)>").unwrap_err(), SkinError::NotAPng);
    }

    #[test]
    fn rejects_truncated_or_corrupt_png() {
        let good = skin_png(64, 64, false);
        assert_eq!(check_skin_png(&good[..good.len() / 2]).unwrap_err(), SkinError::NotAPng);
        let mut corrupt = good.clone();
        let mid = corrupt.len() / 2;
        corrupt[mid] ^= 0xFF;
        corrupt[mid + 1] ^= 0xFF;
        assert!(check_skin_png(&corrupt).is_err());
    }

    #[test]
    fn rejects_wrong_dimensions() {
        for (w, h) in [(32u32, 32u32), (128, 128), (64, 48), (1, 1)] {
            let png = skin_png(w, h, false);
            assert_eq!(check_skin_png(&png).unwrap_err(), SkinError::BadDimensions { width: w, height: h });
        }
    }

    #[test]
    fn rejects_oversized_files() {
        let mut big = skin_png(64, 64, false);
        big.resize(MAX_SKIN_BYTES + 1, 0);
        assert_eq!(check_skin_png(&big).unwrap_err(), SkinError::TooLarge);
    }

    #[test]
    fn converts_other_color_types_and_strips_metadata() {
        // RGB sin alfa con un chunk de texto: se normaliza a RGBA y se descarta el chunk.
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 64, 64);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.add_text_chunk("Comment".into(), "secreto personal".into()).unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&vec![200u8; 64 * 64 * 3]).unwrap();
        }
        assert!(out.windows(8).any(|w| w == b"Comment\0"), "la prueba necesita el chunk de texto");

        let checked = check_skin_png(&out).unwrap();
        assert!(!checked.png.windows(7).any(|w| w == b"Comment"), "los metadatos se eliminan");
        assert!(!checked.png.windows(16).any(|w| w == b"secreto personal"));

        let decoder = png::Decoder::new(Cursor::new(&checked.png));
        let reader = decoder.read_info().unwrap();
        assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    }
}
