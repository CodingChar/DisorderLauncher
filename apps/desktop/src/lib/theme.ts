import { hoverFromAction, shadeFromBackground, DEFAULT_HOVER } from './mc/color'
import { palette } from './mc/palette.svelte'
import type { ThemeEntry } from './types'

/** Variables CSS que escribe el motor de temas; se limpian al volver a un tema incluido. */
const VARS = [
  '--bg', '--surface', '--surface-alt', '--surface-raised', '--text-primary', '--text-secondary',
  '--border', '--border-strong', '--action', '--action-text',
  '--r-sm', '--r-md', '--r-lg', '--r-xl', '--font-display', '--font-body', '--shadow', '--shadow-soft', 'zoom', '--shade',
]

/**
 * Aplica un tema SOLO como variables CSS. Rust ya valido el contenido (colores #RRGGBB, familias
 * sin caracteres especiales, rangos); aqui no se interpreta nada como codigo ni como HTML.
 */
export function applyThemeObject(theme: ThemeEntry) {
  const root = document.documentElement
  const set = (name: string, value: string) => root.style.setProperty(name, value)
  const c = theme.colors

  set('--bg', c.background)
  set('--surface', c.surface)
  set('--surface-alt', c.surfaceAlt)
  set('--surface-raised', c.surfaceRaised)
  set('--text-primary', c.textPrimary)
  set('--text-secondary', c.textSecondary)
  set('--border', c.border)
  set('--border-strong', c.borderStrong)
  set('--action', c.action)
  set('--action-text', c.actionText)

  set('--r-sm', `${theme.shape.inputRadius}px`)
  set('--r-md', `${theme.shape.buttonRadius}px`)
  set('--r-lg', `${theme.shape.cardRadius}px`)
  set('--r-xl', `${theme.shape.heroRadius}px`)

  set('--font-display', `"${theme.typography.displayFamily}", "IBM Plex Sans", system-ui, sans-serif`)
  set('--font-body', `"${theme.typography.bodyFamily}", system-ui, sans-serif`)

  const s = theme.effects.shadowStrength
  const dark = theme.mode === 'dark'
  const alpha = dark ? Math.min(1, s * 1.8) : s * 0.4
  const rgb = dark ? '0, 0, 0' : '10, 10, 10'
  set('--shadow', `0 12px 30px rgba(${rgb}, ${alpha.toFixed(3)})`)
  set('--shadow-soft', `0 4px 14px rgba(${rgb}, ${(alpha * 0.75).toFixed(3)})`)

  // Modo Minecraft: los sprites son PNG fijos, asi que un tema solo tiñe el fondo y el resalte del texto.
  // El zoom del documento (`typography.scale` x `density.uiScale`) ya no se aplica: romperia la rejilla de pixeles del escenario.
  const shade = shadeFromBackground(c.background)
  set('--shade', shade)
  palette.hover = hoverFromAction(c.action, shade)
  root.style.removeProperty('zoom')

  root.dataset.theme = theme.mode
  root.dataset.animations = theme.effects.animations ? 'on' : 'off'
}

export function clearThemeVars() {
  const root = document.documentElement
  for (const name of VARS) root.style.removeProperty(name)
  palette.hover = DEFAULT_HOVER
  delete root.dataset.animations
}
