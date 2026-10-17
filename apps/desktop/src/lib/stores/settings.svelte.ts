import { api } from '../api'
import { applyThemeObject, clearThemeVars } from '../theme'
import type { Settings, ThemeEntry } from '../types'

/** Tema incluido «Azul oscuro»: lo activa el boton de la luna de la cabecera. */
export const BLUE_THEME_ID = 'disorderlaucher.default.blue'

const media = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: light)') : null

class SettingsStore {
  value = $state<Settings>({ schemaVersion: 1, theme: 'system', downloadProfile: 'normal', themeId: null, microsoftClientId: null, javaPath: null })
  dataDir = $state('')
  themes = $state<ThemeEntry[]>([])

  constructor() {
    // Con tema "sistema", seguir los cambios del SO en caliente.
    media?.addEventListener('change', () => {
      if (!this.value.themeId && this.value.theme === 'system') this.applyCurrent()
    })
  }

  /** Aplica el tema personalizado si hay uno elegido y existe; si no, el incluido segun el modo. */
  applyCurrent() {
    const custom = this.value.themeId ? this.themes.find((t) => t.id === this.value.themeId) : undefined
    if (custom) {
      applyThemeObject(custom)
      return
    }
    clearThemeVars()
    const mode = this.value.theme === 'system' ? (media?.matches ? 'light' : 'dark') : this.value.theme
    document.documentElement.dataset.theme = mode
  }

  get isBlue() {
    return this.value.themeId === BLUE_THEME_ID
  }

  /** Alterna entre el tema azul y el tema elegido en Ajustes (claro, oscuro o del sistema). */
  async toggleBlue() {
    await this.save({ ...this.value, themeId: this.isBlue ? null : BLUE_THEME_ID })
  }

  async loadThemes() {
    try {
      this.themes = await api.themesList()
    } catch {
      this.themes = []
    }
  }

  async load() {
    try {
      const snapshot = await api.settingsGet()
      this.value = snapshot.settings
      this.dataDir = snapshot.dataDir
    } catch {
      // Sin ajustes se usan los valores por defecto; no es motivo para bloquear la app.
    }
    await this.loadThemes()
    this.applyCurrent()
  }

  async save(next: Settings) {
    const snapshot = await api.settingsUpdate(next)
    this.value = snapshot.settings
    this.dataDir = snapshot.dataDir
    this.applyCurrent()
  }

  async importTheme() {
    const theme = await api.themesImport()
    if (theme) await this.loadThemes()
    return theme
  }

  async removeTheme(id: string) {
    await api.themesRemove(id)
    await this.loadThemes()
    const snapshot = await api.settingsGet()
    this.value = snapshot.settings
    this.applyCurrent()
  }
}

export const settings = new SettingsStore()
