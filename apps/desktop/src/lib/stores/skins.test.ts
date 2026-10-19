import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { SkinAsset } from '../types'
import { accounts } from './accounts.svelte'
import { SkinsStore } from './skins.svelte'

const asset = (id: string, version = '1'): SkinAsset => ({ id, name: id, model: 'classic', source: 'local', width: 64, height: 64, createdAt: '1', updatedAt: version })
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve() }
beforeEach(() => { accounts.list = [] })
afterEach(() => vi.restoreAllMocks())

it('abre una biblioteca de 500 skins sin leer sus 500 PNG', async () => {
  const store = new SkinsStore()
  vi.spyOn(api, 'skinsList').mockResolvedValue(Array.from({ length: 500 }, (_, i) => asset(String(i))))
  const read = vi.spyOn(api, 'skinsRead')
  await store.load()
  expect(store.loaded).toBe(true)
  expect(store.list).toHaveLength(500)
  expect(read).not.toHaveBeenCalled()
})

it('limita a cuatro las lecturas de las miniaturas y comparte las duplicadas', async () => {
  const store = new SkinsStore()
  store.list = Array.from({ length: 12 }, (_, i) => asset(String(i)))
  let active = 0
  let maximum = 0
  const gates: (() => void)[] = []
  const read = vi.spyOn(api, 'skinsRead').mockImplementation((id) => new Promise((resolve) => {
    maximum = Math.max(maximum, ++active)
    gates.push(() => { active--; resolve(`png:${id}`) })
  }))
  const first = store.ensurePreview('0')
  expect(store.ensurePreview('0')).toBe(first)
  const loading = store.ensurePreviews(store.list.map((item) => item.id))
  await flush()
  expect(read).toHaveBeenCalledTimes(4)
  for (let batch = 0; batch < 3; batch++) {
    gates.splice(0).forEach((done) => done())
    await flush()
  }
  await loading
  expect(maximum).toBe(4)
  expect(read).toHaveBeenCalledTimes(12)
  expect(store.previews['11']).toBe('png:11')
})

it('una lectura antigua no restaura la miniatura anterior a una edicion', async () => {
  const store = new SkinsStore()
  const list = vi.spyOn(api, 'skinsList').mockResolvedValue([asset('a')])
  let old!: (value: string) => void
  const read = vi.spyOn(api, 'skinsRead').mockImplementationOnce(() => new Promise((done) => { old = done })).mockResolvedValue('new-png')
  await store.load()
  const pending = store.ensurePreview('a')
  await flush()
  list.mockResolvedValue([asset('a', '2')])
  await store.load(true)
  const fresh = store.ensurePreview('a')
  old('old-png')
  await Promise.all([pending, fresh])
  expect(store.previews.a).toBe('new-png')
  expect(read).toHaveBeenCalledTimes(2)
})

it('una skin eliminada no reaparece cuando llega un PNG pendiente', async () => {
  const store = new SkinsStore()
  const list = vi.spyOn(api, 'skinsList').mockResolvedValue([asset('a')])
  let resolve!: (value: string) => void
  vi.spyOn(api, 'skinsRead').mockImplementation(() => new Promise((done) => { resolve = done }))
  await store.load()
  const reading = store.ensurePreview('a')
  await flush()
  list.mockResolvedValue([])
  await store.load(true)
  resolve('old-png')
  await reading
  expect(store.previews.a).toBeUndefined()
})

it('entradas simultaneas comparten metadatos; las mutaciones fuerzan su renovacion', async () => {
  const store = new SkinsStore()
  const list = vi.spyOn(api, 'skinsList').mockResolvedValue([asset('a')])
  await Promise.all([store.load(), store.load(), store.load()])
  await store.load()
  expect(list).toHaveBeenCalledTimes(1)
  vi.spyOn(api, 'skinsRename').mockResolvedValue({ ...asset('a', '2'), name: 'Nueva' })
  list.mockResolvedValue([{ ...asset('a', '2'), name: 'Nueva' }])
  await store.rename('a', 'Nueva')
  expect(list).toHaveBeenCalledTimes(2)
  expect(store.list[0].name).toBe('Nueva')
})
