import { afterEach, describe, expect, it, vi } from 'vitest'
import { sessions } from './sessions.svelte'

type Internals = { appendLog: (id: string, l: { stream: 'stdout' | 'stderr'; text: string }) => void }
const store = sessions as unknown as Internals

afterEach(() => {
  vi.useRealTimers()
  sessions.clearLog('a')
})

describe('consola de la sesion', () => {
  it('agrupa las lineas: la lista solo cambia una vez por tanda', () => {
    vi.useFakeTimers()
    for (let i = 0; i < 50; i++) store.appendLog('a', { stream: 'stdout', text: `linea ${i}` })
    expect(sessions.logOf('a')).toHaveLength(0)
    vi.advanceTimersByTime(20)
    const lines = sessions.logOf('a')
    expect(lines).toHaveLength(50)
    expect(lines[0].text).toBe('linea 0')
    // Los numeros son crecientes y unicos (sirven de clave estable).
    expect(new Set(lines.map((l) => l.id)).size).toBe(50)
  })

  it('conserva solo las 1000 ultimas lineas', () => {
    vi.useFakeTimers()
    for (let i = 0; i < 1500; i++) store.appendLog('a', { stream: 'stderr', text: `x${i}` })
    vi.advanceTimersByTime(20)
    const lines = sessions.logOf('a')
    expect(lines).toHaveLength(1000)
    expect(lines[0].text).toBe('x500')
    expect(lines[999].text).toBe('x1499')
  })

  it('limpiar descarta tambien lo que aun no se habia volcado', () => {
    vi.useFakeTimers()
    store.appendLog('a', { stream: 'stdout', text: 'pendiente' })
    sessions.clearLog('a')
    vi.advanceTimersByTime(20)
    expect(sessions.logOf('a')).toHaveLength(0)
  })
})
