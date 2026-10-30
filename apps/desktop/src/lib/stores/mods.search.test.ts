import { afterEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { ContentHit, SearchPage } from '../types'
import { ModsStore, defaultFilters } from './mods.svelte'

afterEach(() => vi.restoreAllMocks())

const hit = (id: string, provider = 'modrinth'): ContentHit => ({
  provider, projectId: id, slug: id, title: id, summary: '', author: 'a', downloads: 1, follows: 1, iconUrl: null,
  categories: [], loaders: [], latestVersion: null, updatedAt: null, clientSide: null, serverSide: null, pageUrl: null,
})
const page = (ids: string[], total: number, offset = 0): SearchPage => ({ hits: ids.map((i) => hit(i)), total, offset })

it('la busqueda envia los filtros y empieza por la primera pagina', async () => {
  const store = new ModsStore()
  const search = vi.spyOn(api, 'modsSearch').mockResolvedValue(page(['a', 'b'], 5))
  await store.search('inst', { ...defaultFilters(), text: 'sod', categories: ['optimization'], side: 'client', sort: 'downloads' })
  expect(search).toHaveBeenCalledWith('inst', { text: 'sod', categories: ['optimization'], side: 'client', sort: 'downloads', compatibility: 'instance', offset: 0 })
  expect(store.results.map((h) => h.projectId)).toEqual(['a', 'b'])
  expect(store.total).toBe(5)
  expect(store.hasMore).toBe(true)
  expect(store.searched).toBe(true)
})

it('cargar mas anade la pagina siguiente sin repetir resultados y respeta los filtros', async () => {
  const store = new ModsStore()
  const search = vi.spyOn(api, 'modsSearch').mockResolvedValueOnce(page(['a', 'b'], 3)).mockResolvedValueOnce(page(['b', 'c'], 3, 2))
  await store.search('inst', { ...defaultFilters(), text: 'x', sort: 'follows' })
  await store.loadMore('inst')
  expect(search.mock.calls[1][1]).toMatchObject({ text: 'x', sort: 'follows', offset: 2 })
  expect(store.results.map((h) => h.projectId)).toEqual(['a', 'b', 'c'])
  expect(store.hasMore).toBe(false)
  await store.loadMore('inst')
  expect(search).toHaveBeenCalledTimes(2)
})

it('un resultado del mismo id de otro proveedor no se considera repetido', async () => {
  const store = new ModsStore()
  vi.spyOn(api, 'modsSearch')
    .mockResolvedValueOnce({ hits: [hit('a')], total: 2, offset: 0 })
    .mockResolvedValueOnce({ hits: [hit('a', 'curseforge')], total: 2, offset: 1 })
  await store.search('inst')
  await store.loadMore('inst')
  expect(store.results.map((h) => `${h.provider}:${h.projectId}`)).toEqual(['modrinth:a', 'curseforge:a'])
})

it('una respuesta vieja no pisa la de una busqueda mas reciente', async () => {
  const store = new ModsStore()
  let slow!: (p: SearchPage) => void
  vi.spyOn(api, 'modsSearch')
    .mockImplementationOnce(() => new Promise((done) => { slow = done }))
    .mockResolvedValueOnce(page(['nuevo'], 1))
  const first = store.search('inst', { ...defaultFilters(), text: 'viejo' })
  await store.search('inst', { ...defaultFilters(), text: 'nuevo' })
  slow(page(['viejo'], 1))
  await first
  expect(store.results.map((h) => h.projectId)).toEqual(['nuevo'])
  expect(store.busy).toBeNull()
})

it('un error al buscar vacia la primera pagina pero conserva lo ya cargado al pedir mas', async () => {
  const store = new ModsStore()
  const search = vi.spyOn(api, 'modsSearch').mockResolvedValueOnce(page(['a'], 3))
  await store.search('inst')
  search.mockRejectedValueOnce(new Error('Sin conexion'))
  await store.loadMore('inst')
  expect(store.results).toHaveLength(1)
  expect(store.error).toBe('Sin conexion')
  search.mockRejectedValueOnce(new Error('Sin conexion'))
  await store.search('inst')
  expect(store.results).toEqual([])
})

it('instalar manda el proveedor del resultado', async () => {
  const store = new ModsStore()
  const install = vi.spyOn(api, 'modsInstall').mockResolvedValue([])
  vi.spyOn(api, 'modsList').mockResolvedValue({ mods: [], missingDependencies: [], conflicts: [], supported: true })
  await store.install('inst', hit('p', 'curseforge'))
  expect(install).toHaveBeenCalledWith('inst', 'p', 'curseforge')
})

it('el catalogo se pide una sola vez y su fallo no rompe la busqueda', async () => {
  const store = new ModsStore()
  const catalog = vi.spyOn(api, 'modsCatalog').mockRejectedValueOnce(new Error('x')).mockResolvedValue({ providers: [], categories: ['library'] })
  await store.loadCatalog()
  expect(store.catalog).toBeNull()
  await store.loadCatalog()
  await store.loadCatalog()
  expect(catalog).toHaveBeenCalledTimes(2)
  expect(store.catalog?.categories).toEqual(['library'])
})
