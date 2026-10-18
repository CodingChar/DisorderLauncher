import { api, events } from '../api'
import type { Instance, InstanceChanges, LoaderKind } from '../types'

class InstancesStore {
  list = $state<Instance[]>([])
  selectedId = $state<string | null>(null)
  loaded = $state(false)
  loadError = $state<string | null>(null)
  /** Avance de la importacion de un modpack en curso (0-100), o `null`. */
  importPercent = $state<number | null>(null)

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

  async create(name: string, version: string, loader: LoaderKind = 'vanilla') {
    const instance = await api.instancesCreate(name, version, loader)
    await this.load()
    this.selectedId = instance.id
    return instance
  }

  /** Crea una instancia con una version personalizada de una carpeta .minecraft (`dir`: `null` = la habitual). */
  async createCustom(name: string, dir: string | null, version: string, copyData: boolean) {
    const instance = await api.instancesCreateCustom(name, dir, version, copyData)
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

  /** Abre el selector de modpack y crea la instancia. `null` si el usuario cancela. */
  async importModpack() {
    this.importPercent = 0
    const stop = await events.onInstallProgress((e) => {
      if (e.phase === 'modpack' && e.total > 0) this.importPercent = Math.round((e.completed / e.total) * 100)
    })
    try {
      const instance = await api.modpacksImport()
      if (instance) {
        await this.load()
        this.selectedId = instance.id
      }
      return instance
    } finally {
      stop()
      this.importPercent = null
    }
  }

  select(id: string) {
    this.selectedId = id
  }
}

export const instances = new InstancesStore()
