import { afterEach, describe, expect, it } from 'vitest'
import { applyThemeObject, clearThemeVars } from './theme'
import type { ThemeEntry } from './types'

function fakeRoot() {
  const style = new Map<string, string>()
  const root = {
    style: { setProperty: (k: string, v: string) => style.set(k, v), removeProperty: (k: string) => void style.delete(k) },
    dataset: {} as Record<string, string>,
  }
  ;(globalThis as { document?: unknown }).document = { documentElement: root }
  return { style, root }
}

const theme = (scale: number, uiScale: number) =>
  ({
    mode: 'dark',
    colors: { background: '#000', surface: '#000', surfaceAlt: '#000', surfaceRaised: '#000', textPrimary: '#fff', textSecondary: '#fff', border: '#111', borderStrong: '#222', action: '#fff', actionText: '#000' },
    typography: { displayFamily: 'A', bodyFamily: 'B', scale },
    shape: { inputRadius: 7, buttonRadius: 8, cardRadius: 13, heroRadius: 18 },
    density: { uiScale, compact: false },
    effects: { shadowStrength: 0.2, animations: true },
  }) as unknown as ThemeEntry

afterEach(() => void delete (globalThis as { document?: unknown }).document)

describe('tema: escala', () => {
  it('multiplica la escala de texto por la de la interfaz', () => {
    const { style } = fakeRoot()
    applyThemeObject(theme(1.2, 1.1))
    expect(style.get('zoom')).toBe('1.320')
  })

  it('no escribe zoom con escala 1 y lo limpia al volver al tema incluido', () => {
    const { style } = fakeRoot()
    applyThemeObject(theme(1, 1))
    expect(style.has('zoom')).toBe(false)
    applyThemeObject(theme(1.3, 1))
    clearThemeVars()
    expect(style.has('zoom')).toBe(false)
  })
})
