import { api } from '../api'
import type { ModsOverview, ProjectHit, UpdateInfo } from '../types'

/** Estado de la pantalla de mods de una instancia. */
class ModsStore {
  overview = $state<ModsOverview | null>(null)
  results = $state<ProjectHit[]>([])
  updates = $state<UpdateInfo[] | null>(null)
  loading = $state(false)
  busy = $state<string | null>(null)
  error = $state<string | null>(null)
  notice = $state<string | null>(null)

  async load(instanceId: string) {
    this.loading = true
    try {
      this.overview = await api.modsList(instanceId)
      this.error = null
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.loading = false
    }
  }

  reset() {
    this.overview = null
    this.results = []
    this.updates = null
    this.error = null
    this.notice = null
    this.busy = null
  }

  /** Ejecuta una accion marcando el elemento como ocupado y recargando la lista. */
  private async run(instanceId: string, key: string, action: () => Promise<string | void>) {
    this.busy = key
    this.error = null
    this.notice = null
    try {
      const message = await action()
      if (message) this.notice = message
      await this.load(instanceId)
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.busy = null
    }
  }

  setEnabled(id: string, file: string, enabled: boolean) {
    return this.run(id, file, () => api.modsSetEnabled(id, file, enabled))
  }

  remove(id: string, file: string) {
    return this.run(id, file, () => api.modsRemove(id, file))
  }

  /** Instala los mods de rendimiento recomendados (Sodium, Lithium...) y resume el resultado. */
  optimize(id: string) {
    return this.run(id, 'optimize', async () => {
      const report = await api.instancesOptimizeMods(id)
      const done = report.installed.length > 0 ? `Instalados: ${report.installed.join(', ')}.` : 'No se instaló nada nuevo.'
      const skipped = report.skipped.length > 0 ? ` No disponibles para esta versión: ${report.skipped.map((s) => s.name).join(', ')}.` : ''
      return done + skipped
    })
  }

  addLocal(id: string) {
    return this.run(id, 'add', async () => {
      const added = await api.modsAddLocal(id)
      return added ? `Se agregó «${added.name}».` : undefined
    })
  }

  async search(id: string, query: string) {
    this.busy = 'search'
    this.error = null
    try {
      this.results = await api.modsSearch(id, query)
    } catch (e) {
      this.error = (e as Error).message
      this.results = []
    } finally {
      this.busy = null
    }
  }

  install(id: string, project: ProjectHit) {
    return this.run(id, project.project_id, async () => {
      const installed = await api.modsInstall(id, project.project_id)
      const extras = installed.filter((m) => m.isDependency).map((m) => m.title)
      if (installed.length === 0) return `«${project.title}» ya estaba instalado.`
      return extras.length
        ? `Se instaló «${project.title}» y también ${extras.join(', ')} (lo necesita para funcionar).`
        : `Se instaló «${project.title}».`
    })
  }

  async checkUpdates(id: string) {
    this.busy = 'updates'
    this.error = null
    try {
      this.updates = await api.modsCheckUpdates(id)
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.busy = null
    }
  }

  update(id: string, file: string) {
    return this.run(id, file, async () => {
      const result = await api.modsUpdate(id, file)
      this.updates = (this.updates ?? []).filter((u) => u.file !== file)
      return `«${result.title}» se actualizó a la versión ${result.versionNumber}.`
    })
  }

  rollback(id: string, file: string) {
    return this.run(id, file, async () => {
      await api.modsRollback(id, file)
      this.updates = null
      return 'Se restauró la versión anterior.'
    })
  }
}

export const mods = new ModsStore()
