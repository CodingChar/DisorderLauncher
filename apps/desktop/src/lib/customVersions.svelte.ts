import { api } from './api'
import type { ExternalScan } from './types'

/** Versiones instaladas fuera del launcher (OptiFine, Forge...) en una carpeta .minecraft (antes dentro de CustomVersionPicker). */
export class CustomVersions {
  /** Id (carpeta) de la version elegida; vacio si no hay ninguna. */
  value = $state('')
  /** Carpeta revisada; `null` = la habitual de este equipo. */
  dir = $state<string | null>(null)
  copyData = $state(false)
  scan = $state<ExternalScan | null>(null)
  loading = $state(false)
  error = $state<string | null>(null)
  private started = false

  private apply(result: ExternalScan, picked: boolean) {
    this.scan = result
    // Con la carpeta habitual se manda `null`; con una elegida a mano, su ruta (el backend la comprueba).
    this.dir = picked ? result.dir : null
    if (!result.versions.some((v) => v.id === this.value)) this.value = result.versions[0]?.id ?? ''
  }

  /** Busca en la carpeta habitual la primera vez que se necesita. */
  async ensureLoaded() {
    if (this.started) return
    this.started = true
    await this.load()
  }

  async load() {
    this.loading = true
    this.error = null
    try {
      this.apply(await api.externalVersionsScan(null), false)
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.loading = false
    }
  }

  async pickFolder() {
    this.error = null
    try {
      const result = await api.externalVersionsPickDir()
      if (result) this.apply(result, true)
    } catch (e) {
      this.error = (e as Error).message
    }
  }
}
