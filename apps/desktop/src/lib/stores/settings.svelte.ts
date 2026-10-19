import { api } from '../api'
import { clearThemeVars } from '../theme'
import type { Settings } from '../types'

/** Minecraft uses fixed sprites and colors, independent of OS or legacy themes. */
class SettingsStore {
  value = $state<Settings>({ schemaVersion: 1, theme: 'dark', downloadProfile: 'normal', themeId: null, microsoftClientId: null, javaPath: null })
  dataDir = $state('')

  applyCurrent() {
    clearThemeVars()
    document.documentElement.dataset.theme = 'dark'
  }

  async load() {
    try {
      const snapshot = await api.settingsGet()
      this.value = { ...snapshot.settings, theme: 'dark', themeId: null }
      this.dataDir = snapshot.dataDir
    } catch { /* Defaults keep startup available. */ }
    this.applyCurrent()
  }

  async save(next: Settings) {
    const snapshot = await api.settingsUpdate({ ...next, theme: 'dark', themeId: null })
    this.value = { ...snapshot.settings, theme: 'dark', themeId: null }
    this.dataDir = snapshot.dataDir
    this.applyCurrent()
  }
}
export const settings = new SettingsStore()
