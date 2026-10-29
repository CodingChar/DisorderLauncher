import { api, events } from '../api'
import type { Instance, InstanceChanges, LoaderKind } from '../types'

class InstancesStore {
  list = $state<Instance[]>([])
  selectedId = $state<string | null>(null)
  loaded = $state(false)
  loadError = $state<string | null>(null)
  /** Avance de la importacion de un modpack en curso (0-100), o `null`. */
  importPercent = $state<number | null>(null)
  /** Texto de la barra de importacion (modpack o instancia .7z). */
  importLabel = $state('Importando modpack')
  /** Avance de la exportacion de una instancia (0-100), o `null`. */
  exportPercent = $state<number | null>(null)
  private revision = 0

  selected = $derived(this.list.find((i) => i.id === this.selectedId) ?? this.list[0] ?? null)

  async load() {
    const revision = ++this.revision
    try {
      const list = await api.instancesList()
      if (revision !== this.revision) return
      this.list = list
      this.loadError = null
    } catch (e) {
      if (revision !== this.revision) return
      this.loadError = (e as Error).message
    } finally {
      if (revision === this.revision) this.loaded = true
    }
  }

  /** El comando devuelve la instancia persistida: no hace falta releer todas las carpetas. */
  private async remember(instance: Instance) {
    if (!this.loaded || this.loadError) {
      await this.load()
      return
    }
    ++this.revision // Una lectura anterior no puede deshacer esta mutacion.
    const index = this.list.findIndex((item) => item.id === instance.id)
    if (index < 0) this.list = [...this.list, instance]
    else this.list = this.list.map((item, i) => i === index ? instance : item)
  }

  async create(name: string, version: string, loader: LoaderKind = 'vanilla') {
    const instance = await api.instancesCreate(name, version, loader)
    await this.remember(instance)
    this.selectedId = instance.id
    return instance
  }

  /** Crea una instancia con una version personalizada de una carpeta .minecraft (`dir`: `null` = la habitual). */
  async createCustom(name: string, dir: string | null, version: string, copyData: boolean) {
    const instance = await api.instancesCreateCustom(name, dir, version, copyData)
    await this.remember(instance)
    this.selectedId = instance.id
    return instance
  }

  async update(id: string, changes: InstanceChanges) {
    const instance = await api.instancesUpdate(id, changes)
    await this.remember(instance)
    return instance
  }

  async remove(id: string) {
    await api.instancesDelete(id)
    if (this.selectedId === id) this.selectedId = null
    if (!this.loaded || this.loadError) await this.load()
    else {
      ++this.revision
      this.list = this.list.filter((item) => item.id !== id)
    }
  }

  async duplicate(id: string) {
    const copy = await api.instancesDuplicate(id)
    await this.remember(copy)
    this.selectedId = copy.id
    return copy
  }

  /** Abre el selector de modpack y crea la instancia. `null` si el usuario cancela. */
  async importModpack() {
    return this.importWith('modpack', 'Importando modpack', () => api.modpacksImport())
  }

  /** Abre el selector de un paquete .7z y crea la instancia. `null` si el usuario cancela. */
  async importDpack() {
    return this.importWith('import7z', 'Importando instancia', () => api.dpackImport())
  }

  /** Guarda la instancia como paquete .7z. `null` si el usuario cancela. */
  async exportDpack(id: string, includeWorlds: boolean) {
    this.exportPercent = 0
    const stop = await events.onInstallProgress((e) => {
      if (e.phase === 'export7z' && e.instanceId === id && e.total > 0) this.exportPercent = Math.round((e.completed / e.total) * 100)
    })
    try {
      return await api.dpackExport(id, includeWorlds)
    } finally {
      stop()
      this.exportPercent = null
    }
  }

  private async importWith(phase: 'modpack' | 'import7z', label: string, run: () => Promise<Instance | null>) {
    this.importPercent = 0
    this.importLabel = label
    const stop = await events.onInstallProgress((e) => {
      if (e.phase === phase && e.total > 0) this.importPercent = Math.round((e.completed / e.total) * 100)
    })
    try {
      const instance = await run()
      if (instance) {
        await this.remember(instance)
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
