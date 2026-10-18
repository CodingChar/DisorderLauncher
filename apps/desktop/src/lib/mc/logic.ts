// Logica pura de listas con scroll (sin DOM) para poder probarla en node. Todo en enteros de GUI.

/** Margen interior de la banda arriba y abajo. */
export const LIST_PAD = 4
export const SCROLLBAR_W = 6
export const MIN_THUMB = 32

export const contentHeight = (count: number, rowHeight: number) => count * rowHeight + LIST_PAD * 2
export const maxScroll = (count: number, rowHeight: number, height: number) => Math.max(0, contentHeight(count, rowHeight) - height)
export const clampScroll = (scroll: number, count: number, rowHeight: number, height: number) =>
  Math.max(0, Math.min(maxScroll(count, rowHeight, height), Math.round(scroll)))

/** Filas que hay que pintar (visibles +- `overscan`); `first > last` si no hay ninguna. */
export function visibleRange(count: number, rowHeight: number, height: number, scroll: number, overscan = 2): { first: number; last: number } {
  if (count <= 0) return { first: 0, last: -1 }
  const first = Math.max(0, Math.floor((scroll - LIST_PAD) / rowHeight) - overscan)
  const last = Math.min(count - 1, Math.ceil((scroll + height - LIST_PAD) / rowHeight) + overscan)
  return { first, last }
}

/** Scroll minimo para que la fila `index` quede entera dentro de la banda. */
export function ensureVisible(index: number, count: number, rowHeight: number, height: number, scroll: number): number {
  const top = LIST_PAD + index * rowHeight
  let next = scroll
  if (top - LIST_PAD < next) next = top - LIST_PAD
  else if (top + rowHeight + LIST_PAD > next + height) next = top + rowHeight + LIST_PAD - height
  return clampScroll(next, count, rowHeight, height)
}

/** Posicion y largo del pulgar de la barra; `null` si todo cabe y no hace falta barra. */
export function thumbGeometry(count: number, rowHeight: number, height: number, scroll: number): { top: number; length: number } | null {
  const content = contentHeight(count, rowHeight)
  if (content <= height) return null
  const length = Math.min(height, Math.max(MIN_THUMB, Math.floor((height * height) / content)))
  const range = maxScroll(count, rowHeight, height)
  const top = range === 0 ? 0 : Math.floor(((height - length) * scroll) / range)
  return { top, length }
}

/** Scroll que corresponde a arrastrar el pulgar hasta `thumbTop`. */
export function scrollFromThumb(thumbTop: number, count: number, rowHeight: number, height: number): number {
  const thumb = thumbGeometry(count, rowHeight, height, 0)
  if (!thumb) return 0
  const travel = height - thumb.length
  if (travel <= 0) return 0
  const ratio = Math.max(0, Math.min(1, thumbTop / travel))
  return clampScroll(ratio * maxScroll(count, rowHeight, height), count, rowHeight, height)
}

/** Incremento de scroll de la rueda: media fila (18 px en filas de 36) por muesca, siempre al menos una muesca. */
export function wheelScroll(deltaY: number, rowHeight: number): number {
  if (!deltaY) return 0
  const notches = Math.max(1, Math.round(Math.abs(deltaY) / 100))
  return Math.sign(deltaY) * notches * Math.max(1, Math.floor(rowHeight / 2))
}

export type ListKey = 'ArrowUp' | 'ArrowDown' | 'PageUp' | 'PageDown' | 'Home' | 'End'

/** Nueva fila seleccionada tras una tecla (`current < 0`: ninguna seleccionada). */
export function moveSelection(current: number, key: ListKey, count: number, pageRows: number): number {
  if (count <= 0) return -1
  const page = Math.max(1, pageRows)
  const clamp = (n: number) => Math.max(0, Math.min(count - 1, n))
  switch (key) {
    case 'ArrowDown':
      return current < 0 ? 0 : clamp(current + 1)
    case 'ArrowUp':
      return current < 0 ? count - 1 : clamp(current - 1)
    case 'PageDown':
      return current < 0 ? clamp(page - 1) : clamp(current + page)
    case 'PageUp':
      return current < 0 ? 0 : clamp(current - page)
    case 'Home':
      return 0
    case 'End':
      return count - 1
  }
}

/** Posiciones Y (dentro de una fila) de cada linea de texto, segun el alto de fila. */
export function lineOffsets(rowHeight: number, lines: number): number[] {
  if (rowHeight >= 36 && lines === 3) return [2, 13, 23]
  const total = lines * 10 - 1
  const start = Math.max(0, Math.floor((rowHeight - total) / 2))
  return Array.from({ length: lines }, (_, i) => start + i * 10)
}
