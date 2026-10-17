import { api } from '../api'
import type { AccountRecord } from '../types'

class AccountsStore {
  list = $state<AccountRecord[]>([])
  defaultId = $state<string | null>(null)
  loaded = $state(false)
  loadError = $state<string | null>(null)

  current = $derived(this.list.find((a) => a.id === this.defaultId) ?? null)

  async load() {
    try {
      const [list, defaultId] = await Promise.all([api.accountsList(), api.accountsDefault()])
      this.list = list
      this.defaultId = defaultId
      this.loadError = null
    } catch (e) {
      this.loadError = (e as Error).message
    } finally {
      this.loaded = true
    }
  }

  async createLocal(name: string) {
    const record = await api.accountsCreateLocal(name)
    await this.load()
    return record
  }

  async setDefault(id: string) {
    await api.accountsSetDefault(id)
    this.defaultId = id
  }
}

export const accounts = new AccountsStore()
