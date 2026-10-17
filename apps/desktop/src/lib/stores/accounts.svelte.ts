import { api, events } from '../api'
import type { AccountRecord, AuthState, MicrosoftStatus } from '../types'

class AccountsStore {
  list = $state<AccountRecord[]>([])
  defaultId = $state<string | null>(null)
  loaded = $state(false)
  loadError = $state<string | null>(null)

  microsoft = $state<MicrosoftStatus>({ enabled: false, loginInProgress: false })
  /** Estado por cuenta (solo se consulta para Microsoft). */
  states = $state<Record<string, AuthState>>({})
  loginMessage = $state<string | null>(null)

  current = $derived(this.list.find((a) => a.id === this.defaultId) ?? null)
  private listening = false

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

  async remove(id: string) {
    await api.accountsRemove(id)
    await this.load()
  }

  async loadMicrosoft() {
    try {
      this.microsoft = await api.microsoftStatus()
    } catch {
      this.microsoft = { enabled: false, loginInProgress: false }
    }
  }

  /** Consulta el estado de las cuentas Microsoft (puede tardar si no hay red). */
  async refreshStates() {
    for (const account of this.list.filter((a) => a.provider === 'microsoft')) {
      api.accountsAuthState(account.id).then(
        (state) => (this.states = { ...this.states, [account.id]: state }),
        () => {},
      )
    }
  }

  async beginLogin() {
    this.loginMessage = null
    await api.accountsMicrosoftBeginLogin()
    this.microsoft = { ...this.microsoft, loginInProgress: true }
  }

  async cancelLogin() {
    await api.accountsMicrosoftCancelLogin()
  }

  async listen() {
    if (this.listening) return
    this.listening = true
    await events.onAuthState((e) => {
      switch (e.state) {
        case 'authorizing':
          this.microsoft = { ...this.microsoft, loginInProgress: true }
          break
        case 'connected':
          this.microsoft = { ...this.microsoft, loginInProgress: false }
          this.loginMessage = `Cuenta «${e.name}» conectada.`
          void this.load().then(() => this.refreshStates())
          break
        case 'cancelled':
          this.microsoft = { ...this.microsoft, loginInProgress: false }
          break
        case 'failed':
          this.microsoft = { ...this.microsoft, loginInProgress: false }
          this.loginMessage = e.message
          break
      }
    })
  }
}

export const accounts = new AccountsStore()
