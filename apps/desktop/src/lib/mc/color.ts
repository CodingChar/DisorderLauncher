// Colores del modo Minecraft: los temas solo cambian el tinte del fondo (--shade) y el color de resalte.

export const DEFAULT_HOVER = '#ffffa0'

function parseHex(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  const n = parseInt(m[1], 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const v = c / 255
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/** Relacion de contraste WCAG entre dos colores `#RRGGBB` (0 si alguno no es valido). */
export function contrastRatio(a: string, b: string): number {
  const ca = parseHex(a)
  const cb = parseHex(b)
  if (!ca || !cb) return 0
  const [hi, lo] = [luminance(ca), luminance(cb)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

/**
 * Tinte del fondo (`r,g,b` para `rgba(var(--shade), .75)`) derivado del color de fondo del tema.
 * Un fondo claro se oscurece: el texto bitmap es blanco y debe seguir leyendose.
 */
export function shadeFromBackground(background: string): string {
  const rgb = parseHex(background)
  if (!rgb) return '0,0,0'
  const factor = luminance(rgb) > 0.18 ? 0.22 : 1
  return rgb.map((c) => Math.round(c * factor)).join(',')
}

/** Color de resalte del texto: el de accion del tema si contrasta >= 4.5 con el fondo oscuro; si no, el amarillo clasico. */
export function hoverFromAction(action: string, shade: string): string {
  const bg = shade.split(',').map((n) => Math.max(0, Math.min(255, Number(n) || 0)))
  const bgHex = `#${bg.map((c) => c.toString(16).padStart(2, '0')).join('')}`
  return /^#[0-9a-f]{6}$/i.test(action) && contrastRatio(action, bgHex) >= 4.5 ? action.toLowerCase() : DEFAULT_HOVER
}
