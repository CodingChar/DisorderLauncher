import { api } from '../api'
import type { SkinAsset, SkinModel } from '../types'
import { accounts } from './accounts.svelte'

export class SkinsStore {
  list = $state<SkinAsset[]>([])
  /** Data URL de cada skin, para las vistas previas. */
  previews = $state<Record<string, string>>({})
  loaded = $state(false)
  error = $state<string | null>(null)
  /** Skin que abre el editor (`null` = una nueva). */
  editingId = $state<string | null>(null)
  private metadata: Promise<void> | null = null
  private loadedAt = 0
  private reads = new Map<string, Promise<string>>()
  private activeReads = 0
  private waiting: (() => void)[] = []

  load(force = false): Promise<void> {
    if (force && this.metadata) return this.metadata.then(() => this.load(true))
    if (this.metadata) return this.metadata
    if (!force && this.loaded && !this.error && Date.now() - this.loadedAt < 5000) return Promise.resolve()
    const request = this.loadMetadata().finally(() => { this.metadata = null })
    this.metadata = request
    return request
  }

  private async loadMetadata() {
    try {
      const list = await api.skinsList()
      const fresh = new Map(list.map((asset) => [asset.id, asset.updatedAt]))
      for (const old of this.list) {
        if (fresh.get(old.id) !== old.updatedAt) this.invalidatePreview(old.id)
      }
      this.list = list
      this.error = null
      this.loadedAt = Date.now()
      // Los perfiles solo necesitan sus caras, no todos los PNG de la biblioteca.
      void this.ensurePreviews(accounts.list.flatMap((account) => account.providerData.type === 'local' && account.providerData.skinId ? [account.providerData.skinId] : []))
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.loaded = true
    }
  }

  invalidatePreview(id: string) {
    delete this.previews[id]
    this.reads.delete(id)
  }

  private acquire(): Promise<void> {
    if (this.activeReads < 4) {
      this.activeReads++
      return Promise.resolve()
    }
    return new Promise((resolve) => this.waiting.push(resolve))
  }

  private release() {
    const next = this.waiting.shift()
    if (next) next()
    else this.activeReads--
  }

  ensurePreview(id: string): Promise<string> {
    if (this.previews[id]) return Promise.resolve(this.previews[id])
    const pending = this.reads.get(id)
    if (pending) return pending
    const asset = this.list.find((item) => item.id === id)
    if (!asset) return Promise.resolve('')
    const version = asset.updatedAt
    const request = this.acquire().then(async () => {
      try {
        if (this.list.find((item) => item.id === id)?.updatedAt !== version) return ''
        const png = await api.skinsRead(id)
        if (this.reads.get(id) !== request || this.list.find((item) => item.id === id)?.updatedAt !== version) return ''
        this.previews[id] = png
        return png
      } catch { return '' } // Un PNG ilegible no bloquea la lista.
      finally {
        if (this.reads.get(id) === request) this.reads.delete(id)
        this.release()
      }
    })
    this.reads.set(id, request)
    return request
  }

  async ensurePreviews(ids: string[]) {
    await Promise.all(ids.map((id) => this.ensurePreview(id)))
  }

  async importSkin() {
    const skin = await api.skinsImport()
    if (skin) await this.load(true)
    return skin
  }

  async rename(id: string, name: string) {
    await api.skinsRename(id, name)
    await this.load(true)
  }

  async setModel(id: string, model: SkinModel) {
    await api.skinsSetModel(id, model)
    await this.load(true)
  }

  async remove(id: string) {
    await api.skinsRemove(id)
    this.invalidatePreview(id)
    await Promise.all([this.load(true), accounts.load()])
  }

  exportSkin(id: string) {
    return api.skinsExport(id)
  }

  async assign(accountId: string, skinId: string | null) {
    await api.accountsSetSkin(accountId, skinId)
    await accounts.load()
  }
}

export const skins = new SkinsStore()
