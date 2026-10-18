/** Posicion absoluta en px de GUI: `x` desde el centro horizontal; `y` desde arriba (o desde abajo con `anchor="bottom"` y `y` negativa). */
export type Anchor = 'top' | 'bottom'

export function posStyle(x: number, y: number, anchor: Anchor = 'top'): string {
  const top = anchor === 'bottom' ? `calc(var(--H) * 1px + ${y}px)` : `${y}px`
  return `left:calc(var(--cx) * 1px + ${x}px);top:${top}`
}

/** Ancho de banda de lista: centrado, par y como mucho 310. */
export const bandWidth = (stageWidth: number) => Math.min(310, (stageWidth - 16) & ~1)
