import { describe, expect, it } from 'vitest'
import { ITEM_ICONS, LEGACY_ICON_IDS, defaultIconFor, findItemIcon, normalizeIconId } from './itemIcons'

describe('iconos de instancia', () => {
  it('cada icono del catalogo tiene imagen y un id valido para el backend', () => {
    expect(ITEM_ICONS.length).toBeGreaterThanOrEqual(9)
    for (const icon of ITEM_ICONS) {
      expect(icon.src, icon.id).toBeTruthy()
      expect(icon.id).toMatch(/^[a-z0-9-]{1,32}$/)
    }
    expect(new Set(ITEM_ICONS.map((i) => i.id)).size).toBe(ITEM_ICONS.length)
  })

  it('los ids antiguos se convierten a un icono que existe', () => {
    for (const [legacy, current] of Object.entries(LEGACY_ICON_IDS)) {
      expect(findItemIcon(legacy)?.id, legacy).toBe(current)
    }
    expect(normalizeIconId('compass')).toBe('ender-pearl')
    expect(normalizeIconId('torch')).toBe('torch')
    expect(normalizeIconId('no-existe')).toBe('')
    expect(normalizeIconId(null)).toBe('')
  })

  it('cada tipo de instancia tiene un icono por defecto', () => {
    for (const loader of ['vanilla', 'fabric', 'quilt', 'forge', 'neoforge'] as const) {
      expect(defaultIconFor(loader), loader).toBeDefined()
    }
  })
})
