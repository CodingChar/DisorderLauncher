import { afterEach, expect, it, vi } from 'vitest'
import { animateVisible } from './animation'

afterEach(() => vi.unstubAllGlobals())

it('limita el dibujo, pausa al ocultar y respeta movimiento reducido en caliente', () => {
  const doc = new EventTarget() as EventTarget & { hidden: boolean }
  doc.hidden = false
  const motion = new EventTarget() as EventTarget & { matches: boolean }
  motion.matches = false
  const callbacks = new Map<number, FrameRequestCallback>()
  let next = 0
  vi.stubGlobal('document', doc)
  vi.stubGlobal('matchMedia', () => motion)
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => { callbacks.set(++next, callback); return next })
  vi.stubGlobal('cancelAnimationFrame', (id: number) => callbacks.delete(id))
  const draw = vi.fn()
  const stop = animateVisible(draw, 30)
  for (let now = 0; now < 1000; now += 1000 / 144) {
    const pending = [...callbacks.values()]
    callbacks.clear()
    pending.forEach((callback) => callback(now))
  }
  expect(draw.mock.calls.length).toBeLessThanOrEqual(31)
  expect(draw.mock.calls.length).toBeGreaterThanOrEqual(24)
  doc.hidden = true
  doc.dispatchEvent(new Event('visibilitychange'))
  expect(callbacks.size).toBe(0)
  doc.hidden = false
  motion.matches = true
  doc.dispatchEvent(new Event('visibilitychange'))
  expect(draw).toHaveBeenLastCalledWith(0, true)
  expect(callbacks.size).toBe(0)
  motion.matches = false
  motion.dispatchEvent(new Event('change'))
  expect(callbacks.size).toBe(1)
  stop()
  expect(callbacks.size).toBe(0)
  motion.dispatchEvent(new Event('change'))
  expect(callbacks.size).toBe(0)
})
