import { api } from '../api'
import type { LoaderKind, VersionCatalog, VersionEntry } from '../types'

export { versionKindLabel, type VersionKind } from '../versionPicker'

class VersionsStore {
  list = $state<VersionEntry[]>([])
  source = $state<VersionCatalog['source']>('online')
  loading = $state(false)
  error = $state<string | null>(null)
  /** Tipo de instancia al que corresponde `list`. */
  loader = $state<LoaderKind | null>(null)
  private request = 0

  /** Versiones de Minecraft que admite `loader` (red al abrir la lista; sin red, cache o lo instalado). */
  async load(loader: LoaderKind) {
    const mine = ++this.request
    this.loading = true
    try {
      const catalog = await api.versionCatalog(loader)
      if (mine !== this.request) return // llego una peticion mas nueva (se cambio el tipo de instancia)
      this.list = catalog.versions
      this.source = catalog.source
      this.loader = loader
      this.error = null
    } catch (e) {
      if (mine !== this.request) return
      this.list = []
      this.loader = loader
      this.error = (e as Error).message
    } finally {
      if (mine === this.request) this.loading = false
    }
  }
}

export const versions = new VersionsStore()
