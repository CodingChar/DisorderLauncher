import { describe, expect, it } from 'vitest'
import {
  SIZE,
  History,
  MC_COLORS,
  blankSkin,
  boxParts,
  faceMaskAt,
  extractPalette,
  faceRects,
  floodFill,
  getPixel,
  groupMask,
  groupOf,
  hexToRgba,
  layerMask,
  linePoints,
  mirrorPixel,
  rgbaToHex,
  setPixel,
  upgradeLegacy,
  type Pixels,
  type Rgba,
} from './skinEditor'

const blank = () => new Uint8ClampedArray(SIZE * SIZE * 4)
const RED: Rgba = [255, 0, 0, 255]

describe('geometria de piezas', () => {
  it('las caras siguen la disposicion estandar de Minecraft', () => {
    const head = boxParts('classic').find((p) => p.name === 'head')!
    const faces = faceRects(head)
    expect(faces.front).toEqual({ x: 8, y: 8, w: 8, h: 8 })
    expect(faces.right).toEqual({ x: 0, y: 8, w: 8, h: 8 })
    expect(faces.left).toEqual({ x: 16, y: 8, w: 8, h: 8 })
    expect(faces.back).toEqual({ x: 24, y: 8, w: 8, h: 8 })
    expect(faces.top).toEqual({ x: 8, y: 0, w: 8, h: 8 })
  })

  it('los brazos slim miden 3 px de ancho y los classic 4', () => {
    const arm = (m: 'classic' | 'slim') => boxParts(m).find((p) => p.name === 'rightArm')!
    expect(faceRects(arm('slim')).front.w).toBe(3)
    expect(faceRects(arm('classic')).front.w).toBe(4)
  })

  it('las mascaras de capa base y exterior no se solapan y cubren pixeles', () => {
    for (const model of ['classic', 'slim'] as const) {
      const base = layerMask(model, 'base')
      const overlay = layerMask(model, 'overlay')
      expect(base.reduce((a, b) => a + b, 0)).toBeGreaterThan(500)
      expect(overlay.reduce((a, b) => a + b, 0)).toBeGreaterThan(500)
      for (let i = 0; i < base.length; i++) expect(base[i] && overlay[i]).toBeFalsy()
    }
  })

  it('todas las piezas caben dentro de la textura', () => {
    for (const model of ['classic', 'slim'] as const) {
      for (const part of boxParts(model)) {
        for (const r of Object.values(faceRects(part))) {
          expect(r.x).toBeGreaterThanOrEqual(0)
          expect(r.y).toBeGreaterThanOrEqual(0)
          expect(r.x + r.w).toBeLessThanOrEqual(SIZE)
          expect(r.y + r.h).toBeLessThanOrEqual(SIZE)
        }
      }
    }
  })
})

describe('espejo', () => {
  it('es una involucion: espejar dos veces devuelve el pixel original', () => {
    for (const model of ['classic', 'slim'] as const) {
      for (let y = 0; y < SIZE; y++) {
        for (let x = 0; x < SIZE; x++) {
          const m = mirrorPixel(model, x, y)
          if (!m) continue
          expect(mirrorPixel(model, m[0], m[1])).toEqual([x, y])
        }
      }
    }
  })

  it('refleja dentro de la cara frontal de la cabeza y entre caras laterales', () => {
    expect(mirrorPixel('classic', 8, 10)).toEqual([15, 10]) // frente de la cabeza
    expect(mirrorPixel('classic', 0, 9)).toEqual([23, 9]) // lado derecho -> lado izquierdo
    expect(mirrorPixel('classic', 63, 0)).toBeNull() // zona sin usar
  })
})

describe('relleno', () => {
  it('rellena solo la region contigua dentro de la mascara', () => {
    const img = blank()
    const mask = layerMask('classic', 'base')
    const changed = floodFill(img, 10, 10, RED, mask)
    // Todo el lienzo es transparente, pero el relleno no sale de la mascara base conectada.
    expect(changed).toBeGreaterThan(0)
    expect(getPixel(img, 10, 10)).toEqual(RED)
    // Un pixel fuera de la mascara (esquina sin usar) no cambia.
    expect(getPixel(img, 63, 0)).toEqual([0, 0, 0, 0])
  })

  it('no hace nada fuera de la mascara, fuera de limites o con el mismo color', () => {
    const img = blank()
    const mask = layerMask('classic', 'base')
    expect(floodFill(img, 63, 0, RED, mask)).toBe(0)
    expect(floodFill(img, -1, 5, RED, mask)).toBe(0)
    expect(floodFill(img, 99, 5, RED, mask)).toBe(0)
    floodFill(img, 10, 10, RED, mask)
    expect(floodFill(img, 10, 10, RED, mask)).toBe(0)
  })

  it('se detiene en pixeles de otro color', () => {
    const img = blank()
    const mask = new Uint8Array(SIZE * SIZE).fill(1)
    for (let y = 0; y < SIZE; y++) setPixel(img, 10, y, [0, 0, 255, 255])
    const changed = floodFill(img, 0, 0, RED, mask)
    expect(changed).toBe(10 * SIZE)
    expect(getPixel(img, 11, 0)).toEqual([0, 0, 0, 0])
  })
})

describe('relleno por cara', () => {
  it('respeta la parte seleccionada en ambas capas y modelos', () => {
    for (const model of ['classic', 'slim'] as const) {
      for (const layer of ['base', 'overlay'] as const) {
        for (const part of boxParts(model).filter((p) => p.layer === layer)) {
          const face = faceRects(part).front
          const img = blank()
          const mask = faceMaskAt(model, layer, face.x, face.y, groupOf(part.name))
          expect(floodFill(img, face.x, face.y, RED, mask)).toBe(face.w * face.h)
          const wrongGroup = groupOf(part.name) === 'head' ? 'body' : 'head'
          expect(floodFill(blank(), face.x, face.y, RED, faceMaskAt(model, layer, face.x, face.y, wrongGroup))).toBe(0)
        }
      }
    }
  })
  it('la mascara de una cara cubre solo ese rectangulo', () => {
    const mask = faceMaskAt('classic', 'base', 10, 10) // frente de la cabeza (8,8,8,8)
    expect(mask.reduce((a, b) => a + b, 0)).toBe(64)
    expect(mask[10 * SIZE + 10]).toBe(1)
    expect(mask[10 * SIZE + 20]).toBe(0) // cara izquierda de la cabeza
  })

  it('un pixel fuera de toda cara o de otra capa da mascara vacia', () => {
    expect(faceMaskAt('classic', 'base', 63, 0).reduce((a, b) => a + b, 0)).toBe(0)
    expect(faceMaskAt('classic', 'overlay', 10, 10).reduce((a, b) => a + b, 0)).toBe(0)
  })

  it('rellenar con la mascara de la cara no invade las caras vecinas', () => {
    const img = blankSkin('classic')
    const changed = floodFill(img, 10, 10, RED, faceMaskAt('classic', 'base', 10, 10))
    expect(changed).toBe(64)
    expect(getPixel(img, 20, 10)[0]).toBe(200) // cara vecina intacta (tono de la plantilla)
  })
})

describe('linea', () => {
  it('une dos puntos sin huecos', () => {
    const pts = linePoints(0, 0, 5, 2)
    expect(pts[0]).toEqual([0, 0])
    expect(pts.at(-1)).toEqual([5, 2])
    for (let i = 1; i < pts.length; i++) {
      expect(Math.abs(pts[i][0] - pts[i - 1][0])).toBeLessThanOrEqual(1)
      expect(Math.abs(pts[i][1] - pts[i - 1][1])).toBeLessThanOrEqual(1)
    }
    expect(linePoints(3, 3, 3, 3)).toEqual([[3, 3]])
  })
})

describe('historial', () => {
  it('un relleno o trazo sin cambios conserva rehacer y no agrega pasos', () => {
    const history = new History()
    const original = blankSkin('classic')
    const painted = new Uint8ClampedArray(original)
    setPixel(painted, 8, 8, RED)
    expect(history.record(original, painted)).toBe(true)
    const undone = history.undo(painted)!
    expect(history.record(undone, new Uint8ClampedArray(undone))).toBe(false)
    expect(history.canUndo).toBe(false)
    expect(history.canRedo).toBe(true)
    expect(history.redo(undone)).toEqual(painted)
    // Las instantaneas no comparten memoria con el documento mutable.
    original.fill(0)
    expect(history.undo(painted)).toEqual(blankSkin('classic'))
  })
  it('deshace y rehace, y un cambio nuevo descarta el futuro', () => {
    const h = new History()
    const a = blank()
    const b = blank()
    setPixel(b, 0, 0, RED)
    h.push(a)
    expect(h.canUndo).toBe(true)

    const undone = h.undo(b)!
    expect(getPixel(undone, 0, 0)).toEqual([0, 0, 0, 0])
    expect(h.canRedo).toBe(true)
    expect(getPixel(h.redo(undone)!, 0, 0)).toEqual(RED)

    h.undo(b)
    h.push(a)
    expect(h.canRedo).toBe(false)
    expect(new History().undo(a)).toBeNull()
  })

  it('respeta el limite de pasos', () => {
    const h = new History(3)
    for (let i = 0; i < 10; i++) h.push(blank())
    let steps = 0
    while (h.undo(blank())) steps++
    expect(steps).toBe(3)
  })
})

describe('skins antiguas y color', () => {
  it('upgradeLegacy copia la mitad superior y espeja el brazo/pierna derechos a la izquierda', () => {
    const legacy = new Uint8ClampedArray(SIZE * 32 * 4)
    setPixel(legacy, 44, 20, RED) // frente del brazo derecho, columna 0
    setPixel(legacy, 20, 20, [0, 255, 0, 255]) // cuerpo
    const out = upgradeLegacy(legacy)

    expect(out.length).toBe(SIZE * SIZE * 4)
    expect(getPixel(out, 20, 20)).toEqual([0, 255, 0, 255])
    expect(getPixel(out, 44, 20)).toEqual(RED)
    // Frente del brazo izquierdo: (36,52) es su columna 0; el pixel del brazo derecho acaba espejado en la ultima columna.
    expect(getPixel(out, 39, 52)).toEqual(RED)
  })

  it('blankSkin rellena solo la capa base', () => {
    const img = blankSkin('classic')
    expect(getPixel(img, 10, 10)[3]).toBe(255)
    expect(getPixel(img, 40, 10)[3]).toBe(0) // sombrero (capa exterior)
  })

  it('convierte hex <-> rgba', () => {
    expect(hexToRgba('#ff8000')).toEqual([255, 128, 0, 255])
    expect(rgbaToHex([255, 128, 0, 255])).toBe('#ff8000')
  })
})

describe('partes del cuerpo', () => {
  it('agrupa cada pieza con su version exterior', () => {
    expect(groupOf('head')).toBe('head')
    expect(groupOf('hat')).toBe('head')
    expect(groupOf('leftSleeve')).toBe('arms')
    expect(groupOf('rightPants')).toBe('legs')
    expect(groupOf('jacket')).toBe('body')
  })

  it('la mascara de un grupo cubre solo sus piezas y los grupos juntos dan la capa completa', () => {
    const count = (m: Uint8Array) => m.reduce((n, v) => n + v, 0)
    const head = groupMask('classic', 'base', 'head')
    // Cabeza 8x8x8: 2*(8*8)+2*(8*8)+2*(8*8) = 6 caras de 64 px.
    expect(count(head)).toBe(6 * 64)
    expect(head[8 * SIZE + 8]).toBe(1) // cara frontal de la cabeza
    expect(head[20 * SIZE + 20]).toBe(0) // cuerpo
    const sum = (['head', 'body', 'arms', 'legs'] as const).reduce((n, g) => n + count(groupMask('classic', 'base', g)), 0)
    expect(sum).toBe(count(layerMask('classic', 'base')))
    expect(count(groupMask('classic', 'base', 'all'))).toBe(count(layerMask('classic', 'base')))
  })
})

describe('paleta del editor', () => {
  it('extrae los colores mas usados ignorando lo transparente', () => {
    const img = new Uint8ClampedArray(SIZE * SIZE * 4) as Pixels
    setPixel(img, 0, 0, [255, 0, 0, 255])
    setPixel(img, 1, 0, [255, 0, 0, 255])
    setPixel(img, 2, 0, [0, 255, 0, 255])
    setPixel(img, 3, 0, [0, 0, 255, 0])
    expect(extractPalette(img)).toEqual(['#ff0000', '#00ff00'])
    expect(extractPalette(img, 1)).toEqual(['#ff0000'])
  })

  it('una textura vacia no tiene paleta y la base de Minecraft son 16 colores distintos', () => {
    expect(extractPalette(new Uint8ClampedArray(SIZE * SIZE * 4) as Pixels)).toEqual([])
    expect(new Set(MC_COLORS).size).toBe(16)
  })
})
