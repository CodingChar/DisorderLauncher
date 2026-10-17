import { api } from '../api'
import type { Instance } from '../types'

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

  select(id: string) {
    this.selectedId = id
  }
}

export const instances = new InstancesStore()
