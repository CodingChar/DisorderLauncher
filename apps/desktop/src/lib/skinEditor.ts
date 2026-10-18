// Logica pura del editor de skins (sin DOM): geometria por partes, mascaras de capa,
// relleno, espejo, conversion de skins antiguas e historial de deshacer/rehacer.

import type { SkinModel } from './types'

/** Pixeles RGBA de la textura (siempre sobre un ArrayBuffer propio, no compartido). */
export type Pixels = Uint8ClampedArray<ArrayBuffer>

export const SIZE = 64
export type Layer = 'base' | 'overlay'
export type Rgba = [number, number, number, number]
export type FaceName = 'right' | 'front' | 'left' | 'back' | 'top' | 'bottom'
export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

/** Pieza del cuerpo en la textura 64x64: origen (u,v) y dimensiones de la caja (ancho, alto, profundo). */
export interface BoxPart {
  name: string
  layer: Layer
  u: number
  v: number
  w: number
  h: number
  d: number
}

export function boxParts(model: SkinModel): BoxPart[] {
  const aw = model === 'slim' ? 3 : 4
  return [
    { name: 'head', layer: 'base', u: 0, v: 0, w: 8, h: 8, d: 8 },
    { name: 'body', layer: 'base', u: 16, v: 16, w: 8, h: 12, d: 4 },
    { name: 'rightArm', layer: 'base', u: 40, v: 16, w: aw, h: 12, d: 4 },
    { name: 'leftArm', layer: 'base', u: 32, v: 48, w: aw, h: 12, d: 4 },
    { name: 'rightLeg', layer: 'base', u: 0, v: 16, w: 4, h: 12, d: 4 },
    { name: 'leftLeg', layer: 'base', u: 16, v: 48, w: 4, h: 12, d: 4 },
    { name: 'hat', layer: 'overlay', u: 32, v: 0, w: 8, h: 8, d: 8 },
    { name: 'jacket', layer: 'overlay', u: 16, v: 32, w: 8, h: 12, d: 4 },
    { name: 'rightSleeve', layer: 'overlay', u: 40, v: 32, w: aw, h: 12, d: 4 },
    { name: 'leftSleeve', layer: 'overlay', u: 48, v: 48, w: aw, h: 12, d: 4 },
    { name: 'rightPants', layer: 'overlay', u: 0, v: 32, w: 4, h: 12, d: 4 },
    { name: 'leftPants', layer: 'overlay', u: 0, v: 48, w: 4, h: 12, d: 4 },
  ]
}

/** Grupos de partes para trabajar de una en una (selector «Parte del cuerpo»). */
export type BodyGroup = 'all' | 'head' | 'body' | 'arms' | 'legs'

export const BODY_GROUPS: { id: BodyGroup; label: string }[] = [
  { id: 'all', label: 'Todo' },
  { id: 'head', label: 'Cabeza' },
  { id: 'body', label: 'Cuerpo' },
  { id: 'arms', label: 'Brazos' },
  { id: 'legs', label: 'Piernas' },
]

const GROUP_OF: Record<string, BodyGroup> = {
  head: 'head', hat: 'head',
  body: 'body', jacket: 'body',
  rightArm: 'arms', leftArm: 'arms', rightSleeve: 'arms', leftSleeve: 'arms',
  rightLeg: 'legs', leftLeg: 'legs', rightPants: 'legs', leftPants: 'legs',
}

/** Nombre en español de cada pieza, tal como se ve en el personaje (derecha = la suya, no la de la pantalla). */
export const PART_LABEL: Record<string, string> = {
  head: 'Cabeza', hat: 'Sombrero',
  body: 'Cuerpo', jacket: 'Chaqueta',
  rightArm: 'Brazo der.', leftArm: 'Brazo izq.', rightSleeve: 'Manga der.', leftSleeve: 'Manga izq.',
  rightLeg: 'Pierna der.', leftLeg: 'Pierna izq.', rightPants: 'Pantalón der.', leftPants: 'Pantalón izq.',
}

export const groupOf = (partName: string): BodyGroup => GROUP_OF[partName] ?? 'all'

/** Pixeles de las piezas de la capa que pertenecen al grupo (`all` = toda la capa). */
export function groupMask(model: SkinModel, layer: Layer, group: BodyGroup): Uint8Array {
  if (group === 'all') return layerMask(model, layer)
  const mask = new Uint8Array(SIZE * SIZE)
  for (const part of boxParts(model).filter((p) => p.layer === layer && groupOf(p.name) === group)) {
    for (const r of Object.values(faceRects(part))) {
      for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) mask[y * SIZE + x] = 1
    }
  }
  return mask
}

/** Las seis caras de una pieza en la textura (disposicion estandar de Minecraft). */
export function faceRects(p: BoxPart): Record<FaceName, Rect> {
  return {
    right: { x: p.u, y: p.v + p.d, w: p.d, h: p.h },
    front: { x: p.u + p.d, y: p.v + p.d, w: p.w, h: p.h },
    left: { x: p.u + p.d + p.w, y: p.v + p.d, w: p.d, h: p.h },
    back: { x: p.u + 2 * p.d + p.w, y: p.v + p.d, w: p.w, h: p.h },
    top: { x: p.u + p.d, y: p.v, w: p.w, h: p.d },
    bottom: { x: p.u + p.d + p.w, y: p.v, w: p.w, h: p.d },
  }
}

const inRect = (r: Rect, x: number, y: number) => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h

/** 1 en los pixeles que pertenecen a alguna cara de las piezas de la capa. */
export function layerMask(model: SkinModel, layer: Layer): Uint8Array {
  const mask = new Uint8Array(SIZE * SIZE)
  for (const part of boxParts(model).filter((p) => p.layer === layer)) {
    for (const r of Object.values(faceRects(part))) {
      for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) mask[y * SIZE + x] = 1
    }
  }
  return mask
}

/** Mascara de la cara (de la capa indicada) que contiene el pixel; vacia si no cae en ninguna. */
export function faceMaskAt(model: SkinModel, layer: Layer, x: number, y: number): Uint8Array {
  const mask = new Uint8Array(SIZE * SIZE)
  for (const part of boxParts(model).filter((p) => p.layer === layer)) {
    for (const r of Object.values(faceRects(part))) {
      if (!inRect(r, x, y)) continue
      for (let j = r.y; j < r.y + r.h; j++) for (let i = r.x; i < r.x + r.w; i++) mask[j * SIZE + i] = 1
      return mask
    }
  }
  return mask
}

/**
 * Pixel simetrico dentro de la misma pieza: frente/espalda/arriba/abajo se reflejan en si mismas
 * y las caras laterales se intercambian. `null` si el pixel no pertenece a ninguna pieza.
 */
export function mirrorPixel(model: SkinModel, x: number, y: number): [number, number] | null {
  for (const part of boxParts(model)) {
    const faces = faceRects(part)
    for (const name of ['front', 'back', 'top', 'bottom'] as const) {
      const r = faces[name]
      if (inRect(r, x, y)) return [r.x + (r.w - 1 - (x - r.x)), y]
    }
    if (inRect(faces.right, x, y)) return [faces.left.x + (faces.left.w - 1 - (x - faces.right.x)), y]
    if (inRect(faces.left, x, y)) return [faces.right.x + (faces.right.w - 1 - (x - faces.left.x)), y]
  }
  return null
}

export const getPixel = (img: Pixels, x: number, y: number): Rgba => {
  const i = (y * SIZE + x) * 4
  return [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

export function setPixel(img: Pixels, x: number, y: number, color: Rgba) {
  const i = (y * SIZE + x) * 4
  img[i] = color[0]
  img[i + 1] = color[1]
  img[i + 2] = color[2]
  img[i + 3] = color[3]
}

const sameColor = (a: Rgba, b: Rgba) => a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3]

/**
 * Relleno por inundacion (4 vecinos) de los pixeles contiguos del mismo color que el inicial,
 * sin salir de la mascara de la capa activa. Devuelve cuantos pixeles cambio.
 */
export function floodFill(img: Pixels, x: number, y: number, color: Rgba, mask: Uint8Array): number {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE || !mask[y * SIZE + x]) return 0
  const target = getPixel(img, x, y)
  if (sameColor(target, color)) return 0

  let changed = 0
  const stack: number[] = [y * SIZE + x]
  while (stack.length > 0) {
    const index = stack.pop()!
    const px = index % SIZE
    const py = (index - px) / SIZE
    if (!mask[index] || !sameColor(getPixel(img, px, py), target)) continue
    setPixel(img, px, py, color)
    changed++
    if (px > 0) stack.push(index - 1)
    if (px < SIZE - 1) stack.push(index + 1)
    if (py > 0) stack.push(index - SIZE)
    if (py < SIZE - 1) stack.push(index + SIZE)
  }
  return changed
}

/** Linea de Bresenham: evita huecos cuando el puntero se mueve rapido. */
export function linePoints(x0: number, y0: number, x1: number, y1: number): [number, number][] {
  const points: [number, number][] = []
  let [x, y] = [x0, y0]
  const dx = Math.abs(x1 - x0)
  const dy = -Math.abs(y1 - y0)
  const sx = x0 < x1 ? 1 : -1
  const sy = y0 < y1 ? 1 : -1
  let err = dx + dy
  for (;;) {
    points.push([x, y])
    if (x === x1 && y === y1) break
    const e2 = 2 * err
    if (e2 >= dy) {
      err += dy
      x += sx
    }
    if (e2 <= dx) {
      err += dx
      y += sy
    }
  }
  return points
}

/**
 * Convierte una skin antigua 64x32 a 64x64: copia la mitad superior y deriva el brazo y la pierna
 * izquierdos espejando los derechos (cada cara a su equivalente).
 */
export function upgradeLegacy(legacy: Pixels): Pixels {
  const out = new Uint8ClampedArray(SIZE * SIZE * 4)
  out.set(legacy.subarray(0, SIZE * 32 * 4))

  const get = (x: number, y: number) => {
    const i = (y * SIZE + x) * 4
    return [legacy[i], legacy[i + 1], legacy[i + 2], legacy[i + 3]] as Rgba
  }
  const parts = boxParts('classic')
  const find = (name: string) => parts.find((p) => p.name === name)!
  const pairs: [BoxPart, BoxPart][] = [
    [find('rightArm'), find('leftArm')],
    [find('rightLeg'), find('leftLeg')],
  ]
  const faceMap: Record<FaceName, FaceName> = { front: 'front', back: 'back', top: 'top', bottom: 'bottom', right: 'left', left: 'right' }

  for (const [from, to] of pairs) {
    const src = faceRects(from)
    const dst = faceRects(to)
    for (const name of Object.keys(faceMap) as FaceName[]) {
      const s = src[name]
      const t = dst[faceMap[name]]
      for (let j = 0; j < s.h; j++) {
        for (let i = 0; i < s.w; i++) setPixel(out, t.x + (t.w - 1 - i), t.y + j, get(s.x + i, s.y + j))
      }
    }
  }
  return out
}

/** Historial de deshacer/rehacer con instantaneas completas (64x64x4 = 16 KB cada una). */
export class History {
  private undoStack: Pixels[] = []
  private redoStack: Pixels[] = []

  constructor(private readonly limit = 60) {}

  /** Guardar el estado *anterior* a un cambio. */
  push(before: Pixels) {
    this.undoStack.push(new Uint8ClampedArray(before))
    if (this.undoStack.length > this.limit) this.undoStack.shift()
    this.redoStack = []
  }

  undo(current: Pixels): Pixels | null {
    const previous = this.undoStack.pop()
    if (!previous) return null
    this.redoStack.push(new Uint8ClampedArray(current))
    return previous
  }

  redo(current: Pixels): Pixels | null {
    const next = this.redoStack.pop()
    if (!next) return null
    this.undoStack.push(new Uint8ClampedArray(current))
    return next
  }

  get canUndo() {
    return this.undoStack.length > 0
  }

  get canRedo() {
    return this.redoStack.length > 0
  }
}

/** Skin inicial: la capa base rellena con un tono neutro para ver las piezas; la exterior transparente. */
export function blankSkin(model: SkinModel, tone: Rgba = [200, 160, 127, 255]): Pixels {
  const img = new Uint8ClampedArray(SIZE * SIZE * 4)
  const mask = layerMask(model, 'base')
  for (let i = 0; i < mask.length; i++) if (mask[i]) setPixel(img, i % SIZE, Math.floor(i / SIZE), tone)
  return img
}

export function hexToRgba(hex: string, alpha = 255): Rgba {
  const v = hex.replace('#', '')
  return [parseInt(v.slice(0, 2), 16), parseInt(v.slice(2, 4), 16), parseInt(v.slice(4, 6), 16), alpha]
}

export function rgbaToHex([r, g, b]: Rgba): string {
  return `#${[r, g, b].map((c) => c.toString(16).padStart(2, '0')).join('')}`
}
