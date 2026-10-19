// Fuente bitmap de Minecraft (ascii.png): anchos de FontRenderer, dibujo con sombra y ajuste de texto.
// Las funciones de medida son puras (se prueban en node); el dibujo solo funciona en el navegador.

const WIDTHS = [
  6,6,6,6,6,6,4,6,6,6,6,6,6,6,6,4, 4,6,7,6,6,6,6,6,6,0,0,0,0,0,0,0,
  4,2,5,6,6,6,6,3,5,5,5,6,2,6,2,6, 6,6,6,6,6,6,6,6,6,6,2,2,5,6,5,6,
  7,6,6,6,6,6,6,6,6,4,6,6,6,6,6,6, 6,6,6,6,6,6,6,6,6,6,6,4,6,4,6,6,
  3,6,6,6,6,6,5,6,6,2,6,5,3,6,6,6, 6,6,6,6,4,6,6,6,6,6,6,5,2,5,7,6,
  6,6,6,6,6,6,6,6,6,6,6,4,6,3,6,6, 6,6,6,6,6,6,6,6,6,6,6,6,6,6,4,6,
  6,3,6,6,6,6,6,6,6,7,6,6,6,2,6,6, 8,9,9,6,6,6,8,8,6,8,8,8,8,8,6,6,
  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9, 9,9,9,9,9,9,9,9,9,9,9,6,9,9,5,9,
  8,7,7,8,7,8,8,8,7,8,8,7,9,9,6,7, 7,7,7,7,9,6,7,8,7,6,6,9,7,6,7,0,
]
// Glifos 128+ de la textura. No cambiar el orden: es el de ascii.png.
const EXTRA = 'ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜø£Ø×ƒáíóúñÑªº¿®¬½¼¡«»'
const ELLIPSIS = '...'

const hasImage = typeof Image !== 'undefined' && typeof document !== 'undefined'
const image = hasImage ? new Image() : null
if (image) image.src = '/assets/ascii.png'
/** Se resuelve cuando la textura de la fuente esta decodificada (nunca rechaza). */
export const fontReady: Promise<void> = image ? image.decode().catch(() => {}) : Promise.resolve()

function glyphIndex(ch: string): number {
  const code = ch.codePointAt(0)!
  if (code < 32) return 32 // saltos de linea y tabuladores dentro de una linea: un espacio
  if (code <= 126) return code
  const i = EXTRA.indexOf(ch)
  if (i >= 0) return 128 + i
  // Sin glifo propio (Á, Ï...): se usa la letra base sin acento; si tampoco, «?».
  const base = ch.normalize('NFD')[0]
  const baseCode = base.codePointAt(0)!
  return baseCode >= 32 && baseCode <= 126 && base !== ch ? baseCode : 63
}

/** Signos que la textura no tiene se sustituyen por equivalentes (… -> ..., · -> -) antes de medir o dibujar. */
const SUBSTITUTIONS: [RegExp, string][] = [
  [/…/g, '...'],
  [/[·•]/g, '-'],
  [/[–—]/g, '-'],
  [/[‘’]/g, "'"],
  [/[“”]/g, '"'],
  [/ /g, ' '],
]

export function normalizeText(text: string | null | undefined): string {
  let out = String(text ?? '')
  for (const [pattern, replacement] of SUBSTITUTIONS) out = out.replace(pattern, replacement)
  return out
}

const charWidth = (ch: string) => WIDTHS[glyphIndex(ch)]

/** Ancho en px de GUI (sin la sombra). */
export function stringWidth(text: string | null | undefined): number {
  let total = 0
  for (const ch of normalizeText(text)) total += charWidth(ch)
  return total
}

const cache = new Map<string, HTMLCanvasElement>()
const CACHE_LIMIT = 2000 // lienzos de ~4 KB: unos 8 MB como maximo
const atlases = new Map<string, HTMLCanvasElement>()

/** Se tine la textura una vez por color, en lugar de crear tres lienzos auxiliares por etiqueta. */
function atlas(color: string): HTMLCanvasElement {
  const cached = atlases.get(color)
  if (cached) return cached
  const canvas = document.createElement('canvas')
  canvas.width = image!.naturalWidth
  canvas.height = image!.naturalHeight
  const ctx = canvas.getContext('2d')!
  ctx.drawImage(image!, 0, 0)
  ctx.globalCompositeOperation = 'source-in'
  ctx.fillStyle = color
  ctx.fillRect(0, 0, canvas.width, canvas.height)
  if (atlases.size >= 32) atlases.delete(atlases.keys().next().value!)
  atlases.set(color, canvas)
  return canvas
}

/** Tine por adelantado la textura (y su sombra) de los colores habituales: la primera etiqueta de cada color ya no paga ese coste. */
export function warmAtlases(colors: readonly string[]) {
  if (!image || !image.complete || !image.naturalWidth) return
  for (const color of colors) {
    const n = parseInt(color.slice(1), 16) || 0
    atlas(`rgb(${((n >> 16) & 252) >> 2},${((n >> 8) & 252) >> 2},${(n & 252) >> 2})`)
    atlas(color)
  }
}

/** Dibuja `text` con sombra (color >> 2). El lienzo devuelto es compartido: no se debe modificar. */
export function renderText(text: string, color = '#ffffff'): HTMLCanvasElement {
  text = normalizeText(text)
  const key = `${color}|${text}`
  const hit = cache.get(key)
  if (hit) return hit

  const width = stringWidth(text)
  const canvas = document.createElement('canvas')
  canvas.width = Math.max(1, width + 1)
  canvas.height = 9
  if (!image || !image.complete || !image.naturalWidth) return canvas // aun sin textura: no se guarda en cache

  const ctx = canvas.getContext('2d')!
  ctx.imageSmoothingEnabled = false
  const n = parseInt(color.slice(1), 16) || 0
  const shadow = `rgb(${((n >> 16) & 252) >> 2},${((n >> 8) & 252) >> 2},${(n & 252) >> 2})`
  for (const [texture, offset] of [[atlas(shadow), 1], [atlas(color), 0]] as const) {
    let x = offset
    for (const ch of text) {
      const i = glyphIndex(ch)
      const advance = WIDTHS[i]
      if (advance > 1) ctx.drawImage(texture, (i % 16) * 8, Math.floor(i / 16) * 8, advance - 1, 8, x, offset, advance - 1, 8)
      x += advance
    }
  }

  if (cache.size >= CACHE_LIMIT) cache.delete(cache.keys().next().value!)
  cache.set(key, canvas)
  return canvas
}

/** Recorta con "..." para que el texto no supere `max` px de GUI (siempre conserva al menos un caracter). */
export function fitText(text: string | null | undefined, max: number): string {
  const s = normalizeText(text)
  if (!max || stringWidth(s) <= max) return s
  const limit = max - stringWidth(ELLIPSIS)
  const chars = [...s]
  let used = 0
  let keep = 0
  for (const ch of chars) {
    const w = charWidth(ch)
    if (used + w > limit) break
    used += w
    keep++
  }
  return chars.slice(0, Math.max(1, keep)).join('') + ELLIPSIS
}

/** Conserva el final del texto (campos de entrada: se ve lo ultimo escrito). */
export function tailText(text: string | null | undefined, max: number): string {
  const chars = [...normalizeText(text)]
  let used = 0
  let start = chars.length
  while (start > 0) {
    const w = charWidth(chars[start - 1])
    if (used + w > max) break
    used += w
    start--
  }
  return chars.slice(Math.min(start, Math.max(0, chars.length - 1))).join('')
}

/**
 * Parte `text` en lineas de como mucho `w` px: respeta `\n`, corta por caracter las palabras mas largas
 * que `w` y, con `maxLines`, acorta la ultima linea con "..." si sobra texto.
 */
export function wrap(text: string | null | undefined, w: number, maxLines = 0): string[] {
  const out: string[] = []
  for (const paragraph of String(text ?? '').split(/\r?\n/)) {
    if (paragraph === '') {
      out.push('')
      continue
    }
    let line = ''
    for (const word of paragraph.split(' ')) {
      if (!word) continue
      const candidate = line ? `${line} ${word}` : word
      if (stringWidth(candidate) <= w) {
        line = candidate
        continue
      }
      if (line) out.push(line)
      line = ''
      if (stringWidth(word) <= w) {
        line = word
        continue
      }
      for (const ch of word) {
        if (line && stringWidth(line + ch) > w) {
          out.push(line)
          line = ch
        } else line += ch
      }
    }
    out.push(line)
  }
  if (maxLines > 0 && out.length > maxLines) {
    const kept = out.slice(0, maxLines - 1)
    kept.push(fitText(out.slice(maxLines - 1).join(' ') + ELLIPSIS, w))
    return kept
  }
  return out
}
