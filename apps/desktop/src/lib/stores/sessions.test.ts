import { afterEach, describe, expect, it, vi } from 'vitest'
import { sessions } from './sessions.svelte'
import { api } from '../api'

type Internals = {
  appendLog: (id: string, l: { stream: 'stdout' | 'stderr'; text: string }) => void
  pending: Map<string, unknown[]>
  flushLogs: () => void
  progress: (id: string, phase: 'installing', event: { phase: 'assets'; completed: number; total: number }) => void
}
const store = sessions as unknown as Internals

afterEach(() => {
  vi.useRealTimers()
  sessions.clearLog('a')
  sessions.byInstance = {}
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
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

  it('acota la memoria aunque la ventana no reciba fotogramas', () => {
    vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1))
    for (let i = 0; i < 20000; i++) store.appendLog('a', { stream: 'stdout', text: `x${i}` })
    expect(store.pending.get('a')!.length).toBeLessThan(2000)
    store.flushLogs()
    const lines = sessions.logOf('a')
    expect(lines).toHaveLength(1000)
    expect(lines[0].text).toBe('x19000')
    expect(lines[999].text).toBe('x19999')
  })
})

describe('estado de la sesion', () => {
  it('ignora solicitudes duplicadas mientras se prepara la instancia', async () => {
    let accept!: () => void
    const launch = vi.spyOn(api, 'instancesLaunch').mockImplementation(() => new Promise<void>((resolve) => { accept = resolve }))
    const repair = vi.spyOn(api, 'instancesRepair')
    const first = sessions.launch('a')
    await sessions.launch('a')
    await sessions.repair('a')
    expect(launch).toHaveBeenCalledTimes(1)
    expect(repair).not.toHaveBeenCalled()
    expect(sessions.get('a')?.phase).toBe('preparing')
    accept()
    await first
  })

  it('un cierre rechazado conserva el proceso como ocupado', async () => {
    sessions.byInstance = { a: { phase: 'running' } }
    const stop = vi.spyOn(api, 'instancesStop').mockRejectedValue(new Error('No se pudo cerrar'))
    await sessions.stop('a')
    expect(sessions.isBusy('a')).toBe(true)
    expect(sessions.get('a')?.message).toBe('No se pudo cerrar')
    await sessions.stop('a')
    expect(stop).toHaveBeenCalledTimes(2)
  })

  it('un progreso tardio no reactiva una sesion terminada', () => {
    sessions.byInstance = { a: { phase: 'failed', message: 'Fallo' } }
    store.progress('a', 'installing', { phase: 'assets', completed: 1, total: 2 })
    expect(sessions.get('a')?.phase).toBe('failed')
    expect(sessions.get('a')?.message).toBe('Fallo')
  })
})
