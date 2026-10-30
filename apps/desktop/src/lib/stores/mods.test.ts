import { afterEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { ModsOverview } from '../types'
import { ModsStore } from './mods.svelte'

const overview = (supported = true): ModsOverview => ({ mods: [], missingDependencies: [], conflicts: [], supported })
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve() }
afterEach(() => vi.restoreAllMocks())

it('al volver muestra la lista inmediatamente mientras comprueba cambios externos', async () => {
  const store = new ModsStore()
  const read = vi.spyOn(api, 'modsList').mockResolvedValue(overview())
  await store.load('a')
  store.reset()
  let resolve!: (value: ModsOverview) => void
  read.mockImplementation(() => new Promise((done) => { resolve = done }))
  store.open('a')
  expect(store.overview?.supported).toBe(true)
  expect(store.loading).toBe(false)
  resolve(overview(false))
  await flush()
  expect(store.overview?.supported).toBe(false)
})

it('la respuesta de otra instancia no sustituye el apartado actual', async () => {
  const store = new ModsStore()
  let resolve!: (value: ModsOverview) => void
  vi.spyOn(api, 'modsList').mockImplementation((id) => id === 'a'
    ? new Promise((done) => { resolve = done }) : Promise.resolve(overview(false)))
  store.open('a')
  store.open('b')
  await flush()
  resolve(overview(true))
  await flush()
  expect(store.overview?.supported).toBe(false)
})

it('una mutacion renueva el cache; al reabrir no recupera la lista anterior', async () => {
  const store = new ModsStore()
  const read = vi.spyOn(api, 'modsList').mockResolvedValue(overview(true))
  vi.spyOn(api, 'modsSetEnabled').mockResolvedValue(undefined)
  await store.load('a')
  read.mockResolvedValue(overview(false))
  await store.setEnabled('a', 'mod.jar', false)
  store.reset()
  read.mockImplementation(() => new Promise(() => {}))
  store.open('a')
  expect(store.overview?.supported).toBe(false)
})

it('un fallo de refresco conserva los datos previos y muestra el error', async () => {
  const store = new ModsStore()
  const read = vi.spyOn(api, 'modsList').mockResolvedValue(overview())
  await store.load('a')
  read.mockRejectedValue(new Error('Lectura fallida'))
  store.open('a')
  await flush()
  expect(store.overview?.supported).toBe(true)
  expect(store.error).toBe('Lectura fallida')
  expect(store.loading).toBe(false)
})

const missing = (): ModsOverview => ({ mods: [], supported: true, conflicts: [], missingDependencies: [{ modName: 'Alpha', id: 'fabric-api', requires: 'Fabric API' }] })
const fixed = (): ModsOverview => ({ mods: [], supported: true, conflicts: [], missingDependencies: [] })

it('si a un mod le falta otro, se instala solo una vez y se avisa', async () => {
  const store = new ModsStore()
  const list = vi.spyOn(api, 'modsList').mockResolvedValueOnce(missing()).mockResolvedValue(fixed())
  const install = vi.spyOn(api, 'modsInstallMissing').mockResolvedValue({
    installed: [{ projectId: 'P7dR8mSH', title: 'Fabric API', versionNumber: '1', file: 'fabric-api.jar', isDependency: true }],
    unresolved: [],
  })
  store.open('a')
  await flush()
  await flush()
  expect(install).toHaveBeenCalledTimes(1)
  expect(store.notice).toContain('Fabric API')
  expect(list).toHaveBeenCalledTimes(2)
  // Aunque la lista siguiera incompleta no se reintenta en bucle.
  list.mockResolvedValue(missing())
  await store.load('a')
  await flush()
  expect(install).toHaveBeenCalledTimes(1)
})

it('sin internet la instalacion automatica no muestra un error', async () => {
  const store = new ModsStore()
  vi.spyOn(api, 'modsList').mockResolvedValue(missing())
  vi.spyOn(api, 'modsInstallMissing').mockRejectedValue(new Error('Necesitas conexion a internet'))
  store.open('a')
  await flush()
  await flush()
  expect(store.error).toBeNull()
  expect(store.busy).toBeNull()
})

it('lo que no se encuentra se cuenta aparte', async () => {
  const store = new ModsStore()
  vi.spyOn(api, 'modsList').mockResolvedValueOnce(missing()).mockResolvedValue(missing())
  vi.spyOn(api, 'modsInstallMissing').mockResolvedValue({ installed: [], unresolved: ['Mod raro'] })
  store.open('a')
  await flush()
  await flush()
  expect(store.notice).toContain('No se encontró para tu versión: Mod raro')
})

it('agregar un mod a mano instala en el mismo paso lo que pide', async () => {
  const store = new ModsStore()
  vi.spyOn(api, 'modsList').mockResolvedValue(fixed())
  vi.spyOn(api, 'modsAddLocal').mockResolvedValue({ fileName: 'a.jar', enabled: true, name: 'Alpha', version: '1', description: null, size: 1, source: null })
  vi.spyOn(api, 'modsInstallMissing').mockResolvedValue({
    installed: [{ projectId: 'P', title: 'Fabric API', versionNumber: '1', file: 'f.jar', isDependency: true }],
    unresolved: [],
  })
  store.open('a')
  await flush()
  await store.addLocal('a')
  expect(store.notice).toContain('Se agregó «Alpha»')
  expect(store.notice).toContain('Fabric API')
})
