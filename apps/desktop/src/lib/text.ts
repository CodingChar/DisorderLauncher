import type { InstallPhase } from './types'

/** Textos para personas no tecnicas: nada de classpath, artifacts ni tokens. */
export const phaseLabel: Record<InstallPhase, string> = {
  metadata: 'Preparando la descarga',
  loader: 'Descargando el cargador de mods',
  modpack: 'Descargando los archivos del modpack',
  client: 'Descargando el juego',
  libraries: 'Descargando componentes',
  assetIndex: 'Preparando recursos',
  assets: 'Descargando recursos (sonidos y texturas)',
  natives: 'Preparando componentes del sistema',
  verify: 'Comprobando archivos',
}

export const providerLabel = { local: 'Offline Gaming', microsoft: 'Microsoft' } as const

export function loaderLabel(loader: string): string {
  return loader === 'vanilla' ? 'Minecraft' : `${loader.charAt(0).toUpperCase()}${loader.slice(1)} · Minecraft`
}

const LOADER_NAMES: Record<string, string> = { vanilla: 'Original', fabric: 'Fabric', quilt: 'Quilt', forge: 'Forge', neoforge: 'NeoForge' }

/** Nombre corto del tipo de instancia para etiquetas. */
export function loaderName(loader: string): string {
  return LOADER_NAMES[loader] ?? loader
}
