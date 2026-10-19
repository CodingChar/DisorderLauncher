import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import type { Instance } from '../types'
import { instances } from './instances.svelte'

const make = (id: string, name = id): Instance => ({
  id, name, minecraftVersion: '1.21', loader: 'vanilla', loaderVersion: null,
  profileId: 'acc_local_test', java: { mode: 'auto' }, memory: { mode: 'auto' },
  icon: null, optimization: 'off',
  createdAt: '2026-10-18T00:00:00Z', updatedAt: '2026-10-18T00:00:00Z',
})
beforeEach(() => {
  instances.list = [make('a')]
  instances.loaded = true
  instances.loadError = null
  instances.selectedId = 'a'
})
afterEach(() => vi.restoreAllMocks())

it('crear, editar, duplicar y borrar no vuelven a leer la biblioteca completa', async () => {
  const list = vi.spyOn(api, 'instancesList')
  vi.spyOn(api, 'instancesCreate').mockResolvedValue(make('b'))
  vi.spyOn(api, 'instancesUpdate').mockResolvedValue(make('b', 'Editada'))
  vi.spyOn(api, 'instancesDuplicate').mockResolvedValue(make('c'))
  vi.spyOn(api, 'instancesDelete').mockResolvedValue(undefined)
  await instances.create('b', '1.21')
  expect(instances.selectedId).toBe('b')
  await instances.update('b', { name: 'Editada' })
  expect(instances.list.find((item) => item.id === 'b')?.name).toBe('Editada')
  await instances.duplicate('b')
  expect(instances.selectedId).toBe('c')
  await instances.remove('b')
  expect(instances.list.map((item) => item.id)).toEqual(['a', 'c'])
  expect(list).not.toHaveBeenCalled()
})

it('una lectura lenta anterior no oculta una instancia que acaba de crearse', async () => {
  let resolve!: (list: Instance[]) => void
  vi.spyOn(api, 'instancesList').mockImplementation(() => new Promise((done) => { resolve = done }))
  vi.spyOn(api, 'instancesCreate').mockResolvedValue(make('b'))
  const reading = instances.load()
  await instances.create('b', '1.21')
  resolve([make('a')])
  await reading
  expect(instances.list.map((item) => item.id)).toEqual(['a', 'b'])
})

it('la primera carga conserva las instancias que aun no estaban en memoria', async () => {
  instances.loaded = false
  instances.list = []
  const list = vi.spyOn(api, 'instancesList').mockResolvedValue([make('a'), make('b')])
  vi.spyOn(api, 'instancesCreate').mockResolvedValue(make('b'))
  await instances.create('b', '1.21')
  expect(instances.list.map((item) => item.id)).toEqual(['a', 'b'])
  expect(list).toHaveBeenCalledTimes(1)
})
