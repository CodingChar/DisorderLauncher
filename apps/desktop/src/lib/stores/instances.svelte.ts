import { api } from '../api'
import type { Instance, InstanceChanges } from '../types'

class InstancesStore {
  list = $state<Instance[]>([])
  selectedId = $state<string | null>(null)
  loaded = $state(false)
  loadError = $state<string | null>(null)

  selected = $derived(this.list.find((i) => i.id === this.selectedId) ?? this.list[0] ?? null)

  async load() {
    try {
      this.list = await api.instancesList()
      this.loadError = null
    } catch (e) {
      this.loadError = (e as Error).message
    } finally {
      this.loaded = true
    }
  }

  async create(name: string, version: string) {
    const instance = await api.instancesCreate(name, version)
    await this.load()
    this.selectedId = instance.id
    return instance
  }

  async update(id: string, changes: InstanceChanges) {
    const instance = await api.instancesUpdate(id, changes)
    await this.load()
    return instance
  }

  async remove(id: string) {
    await api.instancesDelete(id)
    if (this.selectedId === id) this.selectedId = null
    await this.load()
  }

  async duplicate(id: string) {
    const copy = await api.instancesDuplicate(id)
    await this.load()
    this.selectedId = copy.id
    return copy
  }

  select(id: string) {
    this.selectedId = id
  }
}

export const instances = new InstancesStore()
