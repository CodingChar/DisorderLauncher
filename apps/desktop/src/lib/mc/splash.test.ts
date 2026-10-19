import { describe, expect, it } from 'vitest'
import { SPLASH_ANGLE, splashLayout, splashScale } from './splash'

describe('splash de Minecraft', () => {
  it('conserva el giro y los dos latidos por segundo de GuiMainMenu', () => {
    expect(SPLASH_ANGLE).toBeCloseTo(-20 * Math.PI / 180)
    expect(splashScale(0, 100)).toBeCloseTo(180 / 132)
    expect(splashScale(250, 100)).toBeCloseTo(170 / 132)
    expect(splashScale(500, 100)).toBeCloseTo(splashScale(0, 100))
    expect(splashScale(750, 100)).toBeCloseTo(splashScale(250, 100))
    expect(splashScale(1000, 100)).toBeCloseTo(splashScale(0, 100))
    expect(splashScale(250, 100, true)).toBe(splashScale(0, 100))
  })

  it('contiene texto y sombra durante todo el pulso sin pintar la pantalla entera', () => {
    for (const width of [6, 33, 100, 216]) {
      const box = splashLayout(width, 270, 65, 320)
      expect(box.x).toBeGreaterThanOrEqual(0)
      expect(box.x + box.width).toBeLessThanOrEqual(320)
      expect(box.width * box.height).toBeLessThan(320 * 240 * 0.2)
      for (let now = 0; now < 1000; now += 10) {
        for (const x of [-(width >> 1), -(width >> 1) + width + 1]) {
          for (const y of [-8, 1]) {
            const scale = splashScale(now, width)
            const px = box.anchorX + (x * Math.cos(SPLASH_ANGLE) - y * Math.sin(SPLASH_ANGLE)) * scale
            const py = box.anchorY + (x * Math.sin(SPLASH_ANGLE) + y * Math.cos(SPLASH_ANGLE)) * scale
            expect(px).toBeGreaterThanOrEqual(box.x)
            expect(px).toBeLessThanOrEqual(box.x + box.width)
            expect(py).toBeGreaterThanOrEqual(box.y)
            expect(py).toBeLessThanOrEqual(box.y + box.height)
          }
        }
      }
    }
  })
})
