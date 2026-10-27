import { api } from '../api'
import type { DependencyReport, ModsOverview, ProjectHit, UpdateInfo } from '../types'

/** Estado de la pantalla de mods de una instancia. */
export class ModsStore {
  overview = $state<ModsOverview | null>(null)
  results = $state<ProjectHit[]>([])
  updates = $state<UpdateInfo[] | null>(null)
  loading = $state(false)
  busy = $state<string | null>(null)
  error = $state<string | null>(null)
  notice = $state<string | null>(null)
  private cache = new Map<string, ModsOverview>()
  private context = 0
  private request = 0
  private instanceId: string | null = null
  /** Instancias en las que ya se intento instalar dependencias desde que se abrio la pantalla (evita bucles). */
  private triedDependencies = new Set<string>()

  open(instanceId: string) {
    this.triedDependencies.clear()
    this.reset()
    this.instanceId = instanceId
    this.overview = this.cache.get(instanceId) ?? null
    void this.load(instanceId)
  }

  async load(instanceId: string) {
    const context = this.context
    const request = ++this.request
    if (this.instanceId !== instanceId) this.overview = this.cache.get(instanceId) ?? null
    this.instanceId = instanceId
    this.loading = this.overview === null
    try {
      const overview = await api.modsList(instanceId)
      if (context !== this.context || request !== this.request) return
      this.overview = overview
      this.cache.delete(instanceId)
      if (this.cache.size >= 16) this.cache.delete(this.cache.keys().next().value!)
      this.cache.set(instanceId, overview)
      this.error = null
    } catch (e) {
      if (context !== this.context || request !== this.request) return
      this.error = (e as Error).message
    } finally {
      if (context === this.context && request === this.request) this.loading = false
    }
    // Si a algun mod le falta otro, se instala solo: la persona no tiene que saber que existe.
    if (context === this.context && request === this.request && this.overview?.supported && this.overview.missingDependencies.length > 0) {
      void this.installMissing(instanceId)
    }
  }

  /** Resumen para la persona de lo que se instalo y de lo que no se encontro. */
  private dependencyMessage(report: DependencyReport): string | undefined {
    const parts: string[] = []
    if (report.installed.length > 0) {
      parts.push(`Se instalaron solas las dependencias que faltaban: ${report.installed.map((m) => m.title).join(', ')}.`)
    }
    if (report.unresolved.length > 0) {
      parts.push(`No se encontró para tu versión: ${report.unresolved.join(', ')}.`)
    }
    return parts.join(' ') || undefined
  }

  /** Instala de una vez lo que los mods de la instancia necesitan y no esta. Una sola vez por pantalla abierta. */
  async installMissing(instanceId: string) {
    if (this.triedDependencies.has(instanceId) || this.busy !== null) return
    this.triedDependencies.add(instanceId)
    return this.run(instanceId, 'deps', async () => {
      // Es automatico: sin internet no se molesta con un error (la lista ya avisa de lo que falta).
      try {
        return this.dependencyMessage(await api.modsInstallMissing(instanceId))
      } catch {
        return undefined
      }
    })
  }

  reset() {
    this.context++
    this.request++
    this.instanceId = null
    this.loading = false
    this.overview = null
    this.results = []
    this.updates = null
    this.error = null
    this.notice = null
    this.busy = null
  }

  /** Ejecuta una accion marcando el elemento como ocupado y recargando la lista. */
  private async run(instanceId: string, key: string, action: () => Promise<string | void>) {
    const context = this.context
    this.cache.delete(instanceId)
    this.busy = key
    this.error = null
    this.notice = null
    try {
      const message = await action()
      this.cache.delete(instanceId)
      if (context !== this.context || this.instanceId !== instanceId) return
      if (message) this.notice = message
      await this.load(instanceId)
    } catch (e) {
      if (context !== this.context) return
      this.error = (e as Error).message
    } finally {
      if (context === this.context) this.busy = null
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
      if (!added) return undefined
      // Un mod agregado a mano suele pedir otros (p. ej. Fabric API): se instalan en el mismo paso.
      this.triedDependencies.delete(id)
      const extra = await api.modsInstallMissing(id).then((r) => this.dependencyMessage(r), () => undefined)
      return extra ? `Se agregó «${added.name}». ${extra}` : `Se agregó «${added.name}».`
    })
  }

  async search(id: string, query: string) {
    const context = this.context
    this.busy = 'search'
    this.error = null
    try {
      const results = await api.modsSearch(id, query)
      if (context !== this.context) return
      this.results = results
    } catch (e) {
      if (context !== this.context) return
      this.error = (e as Error).message
      this.results = []
    } finally {
      if (context === this.context) this.busy = null
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
    const context = this.context
    this.busy = 'updates'
    this.error = null
    try {
      const updates = await api.modsCheckUpdates(id)
      if (context !== this.context) return
      this.updates = updates
    } catch (e) {
      if (context !== this.context) return
      this.error = (e as Error).message
    } finally {
      if (context === this.context) this.busy = null
    }
  }

  update(id: string, file: string) {
    const context = this.context
    return this.run(id, file, async () => {
      const result = await api.modsUpdate(id, file)
      if (context === this.context) this.updates = (this.updates ?? []).filter((u) => u.file !== file)
      // La version nueva puede pedir mods que la anterior no pedia.
      const extra = await api.modsInstallMissing(id).then((r) => this.dependencyMessage(r), () => undefined)
      const base = `«${result.title}» se actualizó a la versión ${result.versionNumber}.`
      return extra ? `${base} ${extra}` : base
    })
  }

  rollback(id: string, file: string) {
    const context = this.context
    return this.run(id, file, async () => {
      await api.modsRollback(id, file)
      if (context === this.context) this.updates = null
      return 'Se restauró la versión anterior.'
    })
  }
}

export const mods = new ModsStore()
