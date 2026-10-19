import { afterEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { ModsOverview } from '../types'
import { ModsStore } from './mods.svelte'

const overview = (supported = true): ModsOverview => ({ mods: [], missingDependencies: [], supported })
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
