import type { SkinModel } from './types'

/** Rectangulo de origen (en la textura) y destino (en la vista frontal, en pixeles de skin). */
export interface Part {
  sx: number
  sy: number
  sw: number
  sh: number
  dx: number
  dy: number
  flip?: boolean
}

/** Ancho y alto de la vista frontal en pixeles de skin. */
export const FRONT_SIZE = { w: 16, h: 32 }

/**
 * Piezas de la vista frontal de un personaje: capa base y capa exterior (sombrero, chaqueta,
 * mangas, pantalones). Las skins antiguas 64x32 solo tienen un brazo y una pierna que se espejan.
 */
export function frontParts(model: SkinModel, legacy: boolean): { base: Part[]; overlay: Part[] } {
  const armW = model === 'slim' ? 3 : 4
  const rightArmX = model === 'slim' ? 1 : 0

  const base: Part[] = [
    { sx: 8, sy: 8, sw: 8, sh: 8, dx: 4, dy: 0 },
    { sx: 20, sy: 20, sw: 8, sh: 12, dx: 4, dy: 8 },
    { sx: 44, sy: 20, sw: armW, sh: 12, dx: rightArmX, dy: 8 },
    { sx: 4, sy: 20, sw: 4, sh: 12, dx: 4, dy: 20 },
  ]
  if (legacy) {
    base.push(
      { sx: 44, sy: 20, sw: armW, sh: 12, dx: 12, dy: 8, flip: true },
      { sx: 4, sy: 20, sw: 4, sh: 12, dx: 8, dy: 20, flip: true },
    )
    return { base, overlay: [{ sx: 40, sy: 8, sw: 8, sh: 8, dx: 4, dy: 0 }] }
  }
  base.push(
    { sx: 36, sy: 52, sw: armW, sh: 12, dx: 12, dy: 8 },
    { sx: 20, sy: 52, sw: 4, sh: 12, dx: 8, dy: 20 },
  )
  const overlay: Part[] = [
    { sx: 40, sy: 8, sw: 8, sh: 8, dx: 4, dy: 0 },
    { sx: 20, sy: 36, sw: 8, sh: 12, dx: 4, dy: 8 },
    { sx: 44, sy: 36, sw: armW, sh: 12, dx: rightArmX, dy: 8 },
    { sx: 52, sy: 52, sw: armW, sh: 12, dx: 12, dy: 8 },
    { sx: 4, sy: 36, sw: 4, sh: 12, dx: 4, dy: 20 },
    { sx: 4, sy: 52, sw: 4, sh: 12, dx: 8, dy: 20 },
  ]
  return { base, overlay }
}
