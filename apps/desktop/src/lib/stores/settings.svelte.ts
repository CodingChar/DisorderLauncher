import { api } from '../api'
import type { Settings, Theme } from '../types'

const media = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: light)') : null

function applyTheme(theme: Theme) {
  const resolved = theme === 'system' ? (media?.matches ? 'light' : 'dark') : theme
  document.documentElement.dataset.theme = resolved
}

class SettingsStore {
  value = $state<Settings>({ schemaVersion: 1, theme: 'system', downloadProfile: 'normal', javaPath: null })
  dataDir = $state('')

  constructor() {
    // Con tema "sistema", seguir los cambios del SO en caliente.
    media?.addEventListener('change', () => {
      if (this.value.theme === 'system') applyTheme('system')
    })
  }

  async load() {
    try {
      const snapshot = await api.settingsGet()
      this.value = snapshot.settings
      this.dataDir = snapshot.dataDir
    } catch {
      // Sin ajustes se usan los valores por defecto; no es motivo para bloquear la app.
    }
    applyTheme(this.value.theme)
  }

  async save(next: Settings) {
    const snapshot = await api.settingsUpdate(next)
    this.value = snapshot.settings
    this.dataDir = snapshot.dataDir
    applyTheme(this.value.theme)
  }
}

export const settings = new SettingsStore()
