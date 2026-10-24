import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { panoramaMoves, setPanoramaMoves } from './motion'

describe('panoramaMoves', () => {
  beforeEach(() => {
    const data = new Map<string, string>()
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => data.get(k) ?? null,
      setItem: (k: string, v: string) => data.set(k, v),
    })
  })
  afterEach(() => vi.unstubAllGlobals())

  it('gira por defecto y recuerda la eleccion', () => {
    expect(panoramaMoves()).toBe(true)
    setPanoramaMoves(false)
    expect(panoramaMoves()).toBe(false)
    setPanoramaMoves(true)
    expect(panoramaMoves()).toBe(true)
  })

  it('sin almacenamiento sigue girando', () => {
    vi.stubGlobal('localStorage', { getItem: () => { throw Error('bloqueado') }, setItem: () => { throw Error('bloqueado') } })
    expect(panoramaMoves()).toBe(true)
    expect(() => setPanoramaMoves(false)).not.toThrow()
  })
})
