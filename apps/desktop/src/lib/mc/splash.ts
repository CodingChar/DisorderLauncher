export const SPLASH_COLOR = '#ffff00'
export const SPLASH_ANGLE = -20 * Math.PI / 180

/** Latido de GuiMainMenu: abs(sin(t * 2pi) * .1), dos pulsos por segundo. */
export function splashScale(now: number, textWidth: number, still = false): number {
  const pulse = still ? 0 : Math.abs(Math.sin((now % 1000) / 1000 * Math.PI * 2) * 0.1)
  return (1.8 - pulse) * 100 / (textWidth + 32)
}

/** Acota solo la zona que puede pintar el texto durante todo el latido, incluida su sombra. */
export function splashLayout(textWidth: number, x: number, y: number, stageWidth: number) {
  const scale = splashScale(0, textWidth)
  const cos = Math.cos(SPLASH_ANGLE)
  const sin = Math.sin(SPLASH_ANGLE)
  const left = -(textWidth >> 1)
  const corners = [
    [left, -8], [left + textWidth + 1, -8],
    [left, 1], [left + textWidth + 1, 1],
  ].map(([cx, cy]) => ({ x: (cx * cos - cy * sin) * scale, y: (cx * sin + cy * cos) * scale }))
  const minX = Math.floor(Math.min(...corners.map((p) => p.x))) - 2
  const maxX = Math.ceil(Math.max(...corners.map((p) => p.x))) + 2
  const minY = Math.floor(Math.min(...corners.map((p) => p.y))) - 2
  const maxY = Math.ceil(Math.max(...corners.map((p) => p.y))) + 2
  const anchorX = Math.max(2 - minX, Math.min(x, stageWidth - 2 - maxX))
  return {
    x: Math.floor(anchorX + minX), y: Math.floor(y + minY),
    width: maxX - minX, height: maxY - minY,
    anchorX, anchorY: y,
  }
}
