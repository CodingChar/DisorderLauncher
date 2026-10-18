//! Adaptacion de la textura al formato que espera cada version de Minecraft.
//!
//! Hasta 1.7.x el modelo del jugador usa una textura de 64x32 (cabeza, cuerpo, brazo y pierna
//! derechos; los izquierdos se espejan). Desde 1.8 usa 64x64 con brazo y pierna izquierdos propios
//! y una segunda capa. Si la textura no tiene el formato de la version, el modelo lee otras filas
//! de la imagen y la skin sale cortada y con partes vacias.
//!
//! El juego solo convierte las skins que descarga, no las texturas de un resource pack, asi que
//! esa conversion (incluida la de opacidad) la hacemos nosotros al generar el pack.

use crate::{check_skin_png, encode_rgba, SkinError, SkinModel};

const W: usize = 64;

/// Formato de textura que espera la version del juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinLayout {
    /// 64x32, brazos de 4 px: hasta Minecraft 1.7.x.
    Legacy,
    /// 64x64 con segunda capa: desde Minecraft 1.8.
    Modern,
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Las seis caras de una caja de la textura, en la disposicion estandar:
/// derecha, frente, izquierda, espalda, arriba, abajo.
fn faces(u: usize, v: usize, w: usize, h: usize, d: usize) -> [Rect; 6] {
    [
        Rect { x: u, y: v + d, w: d, h },
        Rect { x: u + d, y: v + d, w, h },
        Rect { x: u + d + w, y: v + d, w: d, h },
        Rect { x: u + 2 * d + w, y: v + d, w, h },
        Rect { x: u + d, y: v, w, h: d },
        Rect { x: u + d + w, y: v, w, h: d },
    ]
}

fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * W + x) * 4;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

fn put(img: &mut [u8], x: usize, y: usize, c: [u8; 4]) {
    let i = (y * W + x) * 4;
    img[i..i + 4].copy_from_slice(&c);
}

fn clear(img: &mut [u8], r: Rect) {
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            put(img, x, y, [0, 0, 0, 0]);
        }
    }
}

/// Pone alfa 255 en `x0..x1, y0..y1`, como hace el juego con las partes que nunca son transparentes.
fn set_opaque(img: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize) {
    for y in y0..y1 {
        for x in x0..x1 {
            img[(y * W + x) * 4 + 3] = 255;
        }
    }
}

/// Convierte una skin a 64x32 (`Legacy`). Devuelve `64 * 32 * 4` bytes RGBA.
///
/// Se queda con la mitad superior (cabeza, sombrero, cuerpo, brazo y pierna derechos); la segunda
/// capa y los miembros izquierdos propios se pierden porque ese formato no los tiene. Si el modelo
/// es `Slim`, el brazo de 3 px se ensancha a 4 px para el brazo del juego. Las partes de la capa
/// base se vuelven opacas: en este formato un pixel transparente se veria como un hueco.
pub fn to_legacy(rgba: &[u8], height: u32, model: SkinModel) -> Vec<u8> {
    let mut out = rgba[..W * 32 * 4].to_vec();
    if height >= 64 && model == SkinModel::Slim {
        let slim = faces(40, 16, 3, 12, 4);
        let classic = faces(40, 16, 4, 12, 4);
        let src = out.clone();
        clear(&mut out, Rect { x: 40, y: 16, w: 16, h: 16 });
        // Con 3 px de ancho, la columna central se repite para llegar a 4.
        const STRETCH: [usize; 4] = [0, 1, 1, 2];
        for (s, d) in slim.iter().zip(classic.iter()) {
            for j in 0..d.h {
                for i in 0..d.w {
                    let si = if s.w == d.w { i } else { STRETCH.get(i).copied().unwrap_or(0) };
                    put(&mut out, d.x + i, d.y + j, px(&src, s.x + si, s.y + j));
                }
            }
        }
    }
    set_opaque(&mut out, 0, 0, 32, 16);
    set_opaque(&mut out, 0, 16, 64, 32);
    out
}

/// Convierte una skin 64x32 a 64x64 (`Modern`): copia la mitad superior y deriva el brazo y la
/// pierna izquierdos espejando los derechos. Devuelve `64 * 64 * 4` bytes RGBA.
pub fn to_modern(rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; W * 64 * 4];
    out[..W * 32 * 4].copy_from_slice(&rgba[..W * 32 * 4]);
    // (origen derecho, destino izquierdo)
    for (from, to) in [((40, 16), (32, 48)), ((0, 16), (16, 48))] {
        let src = faces(from.0, from.1, 4, 12, 4);
        let dst = faces(to.0, to.1, 4, 12, 4);
        // derecha<->izquierda se intercambian; el resto se queda en su cara.
        const MAP: [usize; 6] = [2, 1, 0, 3, 4, 5];
        for (k, s) in src.iter().enumerate() {
            let t = dst[MAP[k]];
            for j in 0..s.h {
                for i in 0..s.w {
                    put(&mut out, t.x + (t.w - 1 - i), t.y + j, px(rgba, s.x + i, s.y + j));
                }
            }
        }
    }
    set_opaque(&mut out, 0, 0, 32, 16);
    set_opaque(&mut out, 0, 16, 64, 32);
    set_opaque(&mut out, 16, 48, 48, 64);
    out
}

/// PNG de la skin en el formato de la version. Si ya lo tiene, solo se normaliza.
pub fn adapt_png(png: &[u8], model: SkinModel, layout: SkinLayout) -> Result<Vec<u8>, SkinError> {
    let checked = check_skin_png(png)?;
    match (layout, checked.height) {
        (SkinLayout::Legacy, _) => encode_rgba(&to_legacy(&checked.rgba, checked.height, model), 64, 32),
        (SkinLayout::Modern, 32) => encode_rgba(&to_modern(&checked.rgba), 64, 64),
        (SkinLayout::Modern, _) => Ok(checked.png),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 64x64 donde cada pixel codifica su posicion: (x, y, 0, 255).
    fn coords(height: usize) -> Vec<u8> {
        let mut img = vec![0u8; W * height * 4];
        for y in 0..height {
            for x in 0..W {
                put(&mut img, x, y, [x as u8, y as u8, 7, 255]);
            }
        }
        img
    }

    #[test]
    fn legacy_keeps_the_top_half_and_drops_the_rest() {
        let src = coords(64);
        let out = to_legacy(&src, 64, SkinModel::Classic);
        assert_eq!(out.len(), W * 32 * 4);
        assert_eq!(&out[..], &src[..W * 32 * 4]);
    }

    #[test]
    fn legacy_makes_the_base_layer_opaque_but_leaves_the_hat_alone() {
        let mut src = coords(64);
        for (x, y) in [(10usize, 5usize), (20, 20), (60, 30), (50, 5)] {
            src[(y * W + x) * 4 + 3] = 0;
        }
        let out = to_legacy(&src, 64, SkinModel::Classic);
        let alpha = |x: usize, y: usize| out[(y * W + x) * 4 + 3];
        assert_eq!(alpha(10, 5), 255, "cabeza");
        assert_eq!(alpha(20, 20), 255, "cuerpo");
        assert_eq!(alpha(60, 30), 255, "zona inferior");
        assert_eq!(alpha(50, 5), 0, "el sombrero conserva su transparencia");
    }

    #[test]
    fn legacy_widens_a_slim_arm_to_four_pixels() {
        let src = coords(64);
        let out = to_legacy(&src, 64, SkinModel::Slim);
        // Frente del brazo: 3 columnas de origen (44..47) -> 4 (44..48), sin hueco.
        let col = |x: usize| px(&out, x, 24);
        assert_eq!(col(44)[0], 44);
        assert_eq!(col(45)[0], 45);
        assert_eq!(col(46)[0], 45, "la columna central se repite");
        assert_eq!(col(47)[0], 46);
        // La cara lateral que en slim empieza en 47 pasa a empezar en 48.
        assert_eq!(col(48)[0], 47);
        assert_eq!(col(51)[0], 50);
        // Ningun pixel del brazo queda vacio: todos vienen de la skin (azul = 7), no del borrado (0).
        let blue = |x: usize, y: usize| px(&out, x, y)[2];
        assert!((40..56).all(|x| blue(x, 24) == 7));
        assert!((44..52).all(|x| blue(x, 17) == 7), "arriba y abajo tambien");
    }

    #[test]
    fn modern_mirrors_the_right_limbs_into_the_left_ones() {
        let src = coords(32);
        let out = to_modern(&src);
        assert_eq!(out.len(), W * 64 * 4);
        assert_eq!(&out[..W * 32 * 4], &src[..]);
        // Frente del brazo derecho (44..48, 20) -> frente del izquierdo (36..40, 52), espejado.
        assert_eq!(px(&out, 36, 52)[0], 47);
        assert_eq!(px(&out, 39, 52)[0], 44);
        // Los lados se intercambian y se voltean: derecha del brazo derecho (40) -> izquierda del izquierdo (43).
        assert_eq!(px(&out, 43, 52)[0], 40);
        assert_eq!(px(&out, 35, 52)[0], 48);
        // Pierna: frente (4..8, 20) -> (20..24, 52).
        assert_eq!(px(&out, 20, 52)[0], 7);
        assert_eq!(px(&out, 23, 52)[0], 4);
    }

    #[test]
    fn modern_makes_the_left_limbs_opaque() {
        let mut src = coords(32);
        for a in src.as_chunks_mut::<4>().0 {
            a[3] = 0;
        }
        let out = to_modern(&src);
        assert_eq!(px(&out, 20, 52)[3], 255);
        assert_eq!(px(&out, 40, 20)[3], 255);
        assert_eq!(px(&out, 50, 5)[3], 0, "el sombrero no se toca");
    }

    #[test]
    fn adapt_png_follows_the_requested_layout() {
        let modern = encode_rgba(&coords(64), 64, 64).unwrap();
        let legacy = encode_rgba(&coords(32), 64, 32).unwrap();

        let as_legacy = check_skin_png(&adapt_png(&modern, SkinModel::Classic, SkinLayout::Legacy).unwrap()).unwrap();
        assert_eq!((as_legacy.width, as_legacy.height), (64, 32));
        let kept = check_skin_png(&adapt_png(&legacy, SkinModel::Classic, SkinLayout::Legacy).unwrap()).unwrap();
        assert_eq!((kept.width, kept.height), (64, 32));

        let as_modern = check_skin_png(&adapt_png(&legacy, SkinModel::Classic, SkinLayout::Modern).unwrap()).unwrap();
        assert_eq!((as_modern.width, as_modern.height), (64, 64));
        let same = adapt_png(&modern, SkinModel::Classic, SkinLayout::Modern).unwrap();
        assert_eq!(check_skin_png(&same).unwrap().rgba, check_skin_png(&modern).unwrap().rgba);

        assert!(adapt_png(b"no es png", SkinModel::Classic, SkinLayout::Legacy).is_err());
    }
}
