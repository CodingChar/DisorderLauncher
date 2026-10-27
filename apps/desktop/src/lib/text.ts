import type { InstallPhase } from './types'

/** Textos para personas no tecnicas: nada de classpath, artifacts ni tokens. */
export const phaseLabel: Record<InstallPhase, string> = {
  metadata: 'Preparando la descarga',
  loader: 'Descargando el cargador de mods',
  modpack: 'Descargando los archivos del modpack',
  adopt: 'Optimizando el espacio de los mods',
  export7z: 'Guardando la instancia',
  import7z: 'Importando la instancia',
  client: 'Descargando el juego',
  libraries: 'Descargando componentes',
  assetIndex: 'Preparando recursos',
  assets: 'Descargando recursos (sonidos y texturas)',
  natives: 'Preparando componentes del sistema',
  verify: 'Comprobando archivos',
}

export const providerLabel = { local: 'Offline Gaming', microsoft: 'Microsoft' } as const

const LOADER_NAMES: Record<string, string> = { vanilla: 'Original', fabric: 'Fabric', quilt: 'Quilt', forge: 'Forge', neoforge: 'NeoForge' }

/** Nombre corto del tipo de instancia para etiquetas. */
export function loaderName(loader: string): string {
  return LOADER_NAMES[loader] ?? loader
}

/** Prefijo con el que el launcher guarda las versiones importadas de una carpeta .minecraft. */
const CUSTOM_PREFIX = 'custom-'

export const isCustomVersion = (id: string): boolean => id.startsWith(CUSTOM_PREFIX)

/** Version para mostrar: sin el prefijo interno de las personalizadas. */
export const versionLabel = (id: string): string => (isCustomVersion(id) ? id.slice(CUSTOM_PREFIX.length) : id)

const EXTERNAL_KIND_NAMES: Record<string, string> = {
  optifine: 'OptiFine',
  forge: 'Forge',
  neoforge: 'NeoForge',
  fabric: 'Fabric',
  quilt: 'Quilt',
  liteloader: 'LiteLoader',
  other: 'Personalizada',
}

export const externalKindName = (kind: string): string => EXTERNAL_KIND_NAMES[kind] ?? 'Personalizada'
