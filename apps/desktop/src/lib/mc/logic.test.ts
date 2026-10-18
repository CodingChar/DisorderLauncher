import { describe, expect, it } from 'vitest'
import {
  LIST_PAD,
  clampScroll,
  ensureVisible,
  lineOffsets,
  maxScroll,
  moveSelection,
  scrollFromThumb,
  thumbGeometry,
  visibleRange,
  wheelScroll,
} from './logic'

describe('scroll de listas', () => {
  it('no hay scroll si todo cabe', () => {
    expect(maxScroll(2, 36, 160)).toBe(0)
    expect(thumbGeometry(2, 36, 160, 0)).toBeNull()
  })

  it('limita el scroll a [0, max]', () => {
    const max = maxScroll(100, 36, 160)
    expect(max).toBe(100 * 36 + 2 * LIST_PAD - 160)
    expect(clampScroll(-5, 100, 36, 160)).toBe(0)
    expect(clampScroll(999999, 100, 36, 160)).toBe(max)
    expect(Number.isInteger(clampScroll(12.6, 100, 36, 160))).toBe(true)
  })

  it('calcula las filas visibles con margen y sin salirse de la lista', () => {
    expect(visibleRange(0, 36, 160, 0)).toEqual({ first: 0, last: -1 })
    const r = visibleRange(3000, 24, 120, 2400)
    expect(r.first).toBeLessThanOrEqual(100)
    expect(r.last).toBeGreaterThanOrEqual(104)
    expect(r.last - r.first).toBeLessThan(15)
    expect(visibleRange(5, 36, 400, 0)).toEqual({ first: 0, last: 4 })
  })

  it('ensureVisible deja la fila entera en pantalla', () => {
    const count = 50
    const down = ensureVisible(20, count, 36, 160, 0)
    expect(LIST_PAD + 20 * 36 + 36 + LIST_PAD).toBeLessThanOrEqual(down + 160)
    const up = ensureVisible(1, count, 36, 160, 700)
    expect(up).toBeLessThanOrEqual(36)
    expect(ensureVisible(0, count, 36, 160, 500)).toBe(0)
    expect(ensureVisible(2, count, 36, 160, 0)).toBe(0)
  })

  it('el pulgar tiene un minimo de 32 y recorre toda la barra', () => {
    const top = thumbGeometry(500, 36, 160, 0)!
    expect(top.length).toBe(32)
    expect(top.top).toBe(0)
    const bottom = thumbGeometry(500, 36, 160, maxScroll(500, 36, 160))!
    expect(bottom.top + bottom.length).toBeLessThanOrEqual(160)
    expect(bottom.top).toBe(160 - 32)
  })

  it('arrastrar el pulgar y el scroll son inversos', () => {
    const max = maxScroll(500, 36, 160)
    expect(scrollFromThumb(0, 500, 36, 160)).toBe(0)
    expect(scrollFromThumb(160 - 32, 500, 36, 160)).toBe(max)
    expect(scrollFromThumb(9999, 500, 36, 160)).toBe(max)
    expect(scrollFromThumb(-50, 500, 36, 160)).toBe(0)
  })

  it('la rueda mueve media fila por muesca', () => {
    expect(wheelScroll(100, 36)).toBe(18)
    expect(wheelScroll(-100, 36)).toBe(-18)
    expect(wheelScroll(300, 36)).toBe(54)
    expect(wheelScroll(3, 36)).toBe(18)
    expect(wheelScroll(0, 36)).toBe(0)
    expect(wheelScroll(100, 10)).toBe(5)
  })
})

describe('teclado en listas', () => {
  it('flechas, paginas, inicio y fin', () => {
    expect(moveSelection(-1, 'ArrowDown', 5, 3)).toBe(0)
    expect(moveSelection(-1, 'ArrowUp', 5, 3)).toBe(4)
    expect(moveSelection(2, 'ArrowDown', 5, 3)).toBe(3)
    expect(moveSelection(4, 'ArrowDown', 5, 3)).toBe(4)
    expect(moveSelection(0, 'ArrowUp', 5, 3)).toBe(0)
    expect(moveSelection(0, 'PageDown', 10, 3)).toBe(3)
    expect(moveSelection(8, 'PageDown', 10, 3)).toBe(9)
    expect(moveSelection(5, 'PageUp', 10, 3)).toBe(2)
    expect(moveSelection(5, 'Home', 10, 3)).toBe(0)
    expect(moveSelection(5, 'End', 10, 3)).toBe(9)
  })

  it('con la lista vacia no hay seleccion', () => {
    expect(moveSelection(-1, 'ArrowDown', 0, 3)).toBe(-1)
  })
})

describe('lineas de una fila', () => {
  it('las filas de 36 con 3 lineas usan 2, 13 y 23', () => {
    expect(lineOffsets(36, 3)).toEqual([2, 13, 23])
  })

  it('una linea queda centrada', () => {
    expect(lineOffsets(24, 1)[0]).toBe(7)
    expect(lineOffsets(10, 1)[0]).toBe(0)
  })
})
