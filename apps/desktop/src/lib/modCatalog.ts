// Textos y opciones de la busqueda de contenido. Los ids son los del contrato neutro (`crates/mods/src/catalog.rs`),
// iguales para cualquier proveedor; aqui solo se les pone nombre. Un id sin nombre se muestra tal cual,
// asi un proveedor nuevo con categorias nuevas no rompe la pantalla.
import type { Compatibility, ContentHit, SortKey } from './types'

export const categoryLabels: Record<string, string> = {
  adventure: 'Aventura',
  decoration: 'Decoración',
  economy: 'Economía',
  equipment: 'Equipamiento',
  food: 'Comida',
  'game-mechanics': 'Mecánicas de juego',
  library: 'Librería',
  magic: 'Magia',
  management: 'Gestión',
  minigame: 'Minijuegos',
  mobs: 'Criaturas',
  optimization: 'Optimización',
  social: 'Social',
  storage: 'Almacenamiento',
  technology: 'Tecnología',
  transportation: 'Transporte',
  utility: 'Utilidades',
  worldgen: 'Generación de mundo',
}

export const categoryLabel = (id: string) => categoryLabels[id] ?? id

export const sortLabels: Record<SortKey, string> = {
  relevance: 'Relevancia',
  downloads: 'Descargas',
  follows: 'Seguidores',
  newest: 'Más nuevos',
  updated: 'Actualizados',
}

export const compatibilityOptions: { value: Compatibility; label: string }[] = [
  { value: 'instance', label: 'Esta instancia' },
  { value: 'anyLoader', label: 'Cualquier loader' },
  { value: 'anyVersion', label: 'Cualquier versión' },
  { value: 'any', label: 'Todo' },
]

export type SideFilter = 'any' | 'client' | 'server'

export const sideOptions: { value: SideFilter; label: string }[] = [
  { value: 'any', label: 'Cualquiera' },
  { value: 'client', label: 'Cliente' },
  { value: 'server', label: 'Servidor' },
]

const loaderLabels: Record<string, string> = { fabric: 'Fabric', forge: 'Forge', neoforge: 'NeoForge', quilt: 'Quilt' }
export const loaderLabel = (id: string) => loaderLabels[id] ?? id

export function formatDownloads(n: number) {
  return n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1).replace(/\.0$/, '')} M` : n >= 1000 ? `${Math.round(n / 1000)} mil` : String(n)
}

/** «hoy», «hace 3 días», «hace 2 meses»... a partir de una fecha ISO. */
export function sinceLabel(iso: string | null, now = Date.now()): string | null {
  if (!iso) return null
  const time = Date.parse(iso)
  if (Number.isNaN(time)) return null
  const days = Math.max(0, Math.floor((now - time) / 86_400_000))
  if (days === 0) return 'hoy'
  if (days === 1) return 'ayer'
  if (days < 30) return `hace ${days} días`
  const months = Math.floor(days / 30)
  if (months < 12) return `hace ${months} ${months === 1 ? 'mes' : 'meses'}`
  const years = Math.floor(days / 365)
  return `hace ${years} ${years === 1 ? 'año' : 'años'}`
}

/** «Cliente y servidor», «Solo cliente»... según lo que declare el proveedor. */
export function environmentLabel(hit: Pick<ContentHit, 'clientSide' | 'serverSide'>): string | null {
  const client = hit.clientSide !== null && hit.clientSide !== 'unsupported'
  const server = hit.serverSide !== null && hit.serverSide !== 'unsupported'
  if (client && server) return 'Cliente y servidor'
  if (client) return 'Solo cliente'
  if (server) return 'Solo servidor'
  return null
}
