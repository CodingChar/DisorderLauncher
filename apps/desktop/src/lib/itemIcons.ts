/**
 * Iconos de objetos para elegir como logo de una instancia: los del paquete `minecraft_iconos_emblematicos`
 * (PNG de pixel art, reescalados sin suavizado a 64 px). Son los unicos iconos del launcher; nada se dibuja
 * por codigo ni se pide a internet. El `id` es lo que se guarda en la instancia (`icon`).
 *
 * Nota: el icono «diamante» del paquete llego vacio (transparente) y no se incluye.
 */

import type { LoaderKind } from './types'

const files = import.meta.glob('../assets/instance-icons/*.png', { eager: true, import: 'default', query: '?url' }) as Record<string, string>

export interface ItemIcon {
  id: string
  label: string
  /** URL de la imagen (puede ir incrustada en la app). */
  src: string
}

/** Orden y nombre visible de cada icono; el archivo se llama `<id>.png`. */
const CATALOG: [id: string, label: string][] = [
  ['grass-block', 'Bloque de césped'],
  ['diamond-sword', 'Espada de diamante'],
  ['diamond-pickaxe', 'Pico de diamante'],
  ['crafting-table', 'Mesa de crafteo'],
  ['torch', 'Antorcha'],
  ['redstone', 'Redstone'],
  ['ender-pearl', 'Perla de Ender'],
  ['golden-apple', 'Manzana dorada'],
  ['tnt', 'TNT'],
]

export const ITEM_ICONS: ItemIcon[] = CATALOG.flatMap(([id, label]) => {
  const src = files[`../assets/instance-icons/${id}.png`]
  return src ? [{ id, label, src }] : []
})

/** Iconos de versiones anteriores del launcher -> el mas parecido del paquete actual. */
export const LEGACY_ICON_IDS: Record<string, string> = {
  'iron-pickaxe': 'diamond-pickaxe',
  apple: 'golden-apple',
  diamond: 'diamond-sword',
  emerald: 'ender-pearl',
  bow: 'diamond-sword',
  compass: 'ender-pearl',
  book: 'crafting-table',
  potion: 'golden-apple',
  heart: 'golden-apple',
  chest: 'crafting-table',
}

/** Id vigente de un icono guardado (o cadena vacia si ya no existe). */
export function normalizeIconId(id: string | null | undefined): string {
  if (!id) return ''
  const current = LEGACY_ICON_IDS[id] ?? id
  return ITEM_ICONS.some((i) => i.id === current) ? current : ''
}

export function findItemIcon(id: string | null | undefined): ItemIcon | undefined {
  const current = normalizeIconId(id)
  return current ? ITEM_ICONS.find((i) => i.id === current) : undefined
}

/** Icono por defecto de cada tipo de instancia (cuando la persona no elige uno). */
const DEFAULT_ICON: Record<LoaderKind, string> = {
  vanilla: 'grass-block',
  fabric: 'diamond-pickaxe',
  quilt: 'torch',
  forge: 'crafting-table',
  neoforge: 'redstone',
}

export function defaultIconFor(loader: LoaderKind): ItemIcon | undefined {
  return findItemIcon(DEFAULT_ICON[loader])
}
