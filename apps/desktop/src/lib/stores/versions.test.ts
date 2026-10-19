import { afterEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { VersionCatalog } from '../types'
import { VersionsStore } from './versions.svelte'

const catalog = (id: string): VersionCatalog => ({ source: 'online', versions: [{ id, type: 'release', releaseTime: '2026-10-18' }] })
afterEach(() => vi.restoreAllMocks())

it('precarga y apertura comparten una sola peticion, y reabrir reutiliza el catalogo', async () => {
  const store = new VersionsStore()
  let resolve!: (value: VersionCatalog) => void
  const read = vi.spyOn(api, 'versionCatalog').mockImplementation(() => new Promise((done) => { resolve = done }))
  const prefetch = store.prefetch('vanilla')
  const opening = store.load('vanilla')
  resolve(catalog('1.21'))
  await Promise.all([prefetch, opening])
  await store.load('vanilla')
  expect(read).toHaveBeenCalledTimes(1)
  expect(store.list[0].id).toBe('1.21')
  expect(store.loading).toBe(false)
})

it('cambiar de loader no mezcla respuestas tardias ni sus caches', async () => {
  const store = new VersionsStore()
  let resolve!: (value: VersionCatalog) => void
  const read = vi.spyOn(api, 'versionCatalog').mockImplementation((loader) => loader === 'fabric'
    ? new Promise((done) => { resolve = done }) : Promise.resolve(catalog('vanilla')))
  const fabric = store.load('fabric')
  await store.load('vanilla')
  resolve(catalog('fabric'))
  await fabric
  expect(store.loader).toBe('vanilla')
  expect(store.list[0].id).toBe('vanilla')
  await store.load('fabric')
  expect(store.list[0].id).toBe('fabric')
  expect(read).toHaveBeenCalledTimes(2)
})

it('un cache vencido se muestra de inmediato y se conserva si falla la red', async () => {
  const store = new VersionsStore()
  const clock = vi.spyOn(Date, 'now').mockReturnValue(1000)
  const read = vi.spyOn(api, 'versionCatalog').mockResolvedValue(catalog('1.21'))
  await store.load('vanilla')
  clock.mockReturnValue(62_000)
  read.mockRejectedValue(new Error('Sin red'))
  const refresh = store.load('vanilla')
  expect(store.list[0].id).toBe('1.21')
  expect(store.loading).toBe(false)
  await refresh
  expect(store.list[0].id).toBe('1.21')
  expect(store.source).toBe('cached')
  expect(store.error).toBe('Sin red')
})

it('la accion explicita de actualizar ignora el TTL', async () => {
  const store = new VersionsStore()
  const read = vi.spyOn(api, 'versionCatalog').mockResolvedValue(catalog('1.20'))
  await store.load('vanilla')
  read.mockResolvedValue(catalog('1.21'))
  await store.load('vanilla', true)
  expect(store.list[0].id).toBe('1.21')
  expect(read).toHaveBeenCalledTimes(2)
})
