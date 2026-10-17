import { api } from '../api'
import type { VersionEntry } from '../types'

class VersionsStore {
  list = $state<VersionEntry[]>([])
  loading = $state(false)
  error = $state<string | null>(null)

  async load() {
    this.loading = true
    try {
      this.list = await api.listVersions()
      this.error = null
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.loading = false
    }
  }
}

export const versions = new VersionsStore()
