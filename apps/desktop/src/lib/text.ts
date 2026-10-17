import type { InstallPhase } from './types'

/** Textos para personas no tecnicas: nada de classpath, artifacts ni tokens. */
export const phaseLabel: Record<InstallPhase, string> = {
  metadata: 'Preparando la descarga',
  client: 'Descargando el juego',
  libraries: 'Descargando componentes',
  assetIndex: 'Preparando recursos',
  assets: 'Descargando recursos (sonidos y texturas)',
  natives: 'Preparando componentes del sistema',
  verify: 'Comprobando archivos',
}

export const providerLabel = { local: 'Offline Gaming', microsoft: 'Microsoft' } as const

export function loaderLabel(loader: string): string {
  return loader === 'vanilla' ? 'Minecraft' : loader.charAt(0).toUpperCase() + loader.slice(1)
}
