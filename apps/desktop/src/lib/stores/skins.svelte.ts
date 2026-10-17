import { api } from '../api'
import type { SkinAsset, SkinModel } from '../types'
import { accounts } from './accounts.svelte'

class SkinsStore {
  list = $state<SkinAsset[]>([])
  /** Data URL de cada skin, para las vistas previas. */
  previews = $state<Record<string, string>>({})
  loaded = $state(false)
  error = $state<string | null>(null)
  /** Skin que abre el editor (`null` = una nueva). */
  editingId = $state<string | null>(null)

  async load() {
    try {
      this.list = await api.skinsList()
      this.error = null
      await Promise.all(
        this.list
          .filter((s) => !this.previews[s.id])
          .map(async (s) => {
            try {
              this.previews = { ...this.previews, [s.id]: await api.skinsRead(s.id) }
            } catch {
              // Una vista previa fallida no impide mostrar la skin.
            }
          }),
      )
    } catch (e) {
      this.error = (e as Error).message
    } finally {
      this.loaded = true
    }
  }

  async importSkin() {
    const skin = await api.skinsImport()
    if (skin) await this.load()
    return skin
  }

  async rename(id: string, name: string) {
    await api.skinsRename(id, name)
    await this.load()
  }

  async setModel(id: string, model: SkinModel) {
    await api.skinsSetModel(id, model)
    await this.load()
  }

  async remove(id: string) {
    await api.skinsRemove(id)
    const { [id]: _gone, ...rest } = this.previews
    this.previews = rest
    await Promise.all([this.load(), accounts.load()])
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
