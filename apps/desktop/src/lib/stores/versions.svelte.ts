import { api } from '../api'
import type { LoaderKind, VersionCatalog, VersionEntry } from '../types'

export { versionKindLabel, type VersionKind } from '../versionPicker'

export class VersionsStore {
  list = $state<VersionEntry[]>([])
  source = $state<VersionCatalog['source']>('online')
  loading = $state(false)
  error = $state<string | null>(null)
  /** Tipo de instancia al que corresponde `list`. */
  loader = $state<LoaderKind | null>(null)
  private request = 0
  private cache = new Map<LoaderKind, { catalog: VersionCatalog; at: number }>()
  private pending = new Map<LoaderKind, Promise<VersionCatalog>>()

  private fetch(loader: LoaderKind): Promise<VersionCatalog> {
    const existing = this.pending.get(loader)
    if (existing) return existing
    const request = api.versionCatalog(loader).then((catalog) => {
      this.cache.set(loader, { catalog, at: Date.now() })
      return catalog
    }).finally(() => this.pending.delete(loader))
    this.pending.set(loader, request)
    return request
  }

  async prefetch(loader: LoaderKind) {
    const hit = this.cache.get(loader)
    if (hit && Date.now() - hit.at < 60_000) return
    try { await this.fetch(loader) } catch { /* La apertura de la pantalla conserva su manejo de errores. */ }
  }

  /** Versiones de Minecraft que admite `loader` (red al abrir la lista; sin red, cache o lo instalado). */
  async load(loader: LoaderKind, force = false) {
    const mine = ++this.request
    const hit = this.cache.get(loader)
    this.error = null
    this.loader = loader
    this.list = hit?.catalog.versions ?? []
    this.source = hit?.catalog.source ?? 'online'
    this.loading = !hit
    if (hit && !force && Date.now() - hit.at < 60_000) return
    try {
      const catalog = await this.fetch(loader)
      if (mine !== this.request) return // llego una peticion mas nueva (se cambio el tipo de instancia)
      this.list = catalog.versions
      this.source = catalog.source
      this.loader = loader
      this.error = null
    } catch (e) {
      if (mine !== this.request) return
      if (!hit) this.list = []
      else this.source = 'cached'
      this.loader = loader
      this.error = (e as Error).message
    } finally {
      if (mine === this.request) this.loading = false
    }
  }
}

export const versions = new VersionsStore()
