import { preloadSfx } from './audio/sfx'
import { fontReady, warmAtlases } from './mc/font'

// Texturas que la interfaz necesita casi siempre. Se descargan y decodifican antes de montar la app y se
// retienen (unos 30-40 MB ya decodificadas) para que el panorama y los botones no se dibujen a trozos.
const TEXTURES = [
  ...[0, 1, 2, 3, 4, 5].map((i) => `panorama/panorama_${i}.png`),
  'widgets.png',
  'button.png',
  'button-hover.png',
  'options_background.png',
  'world_selection.png',
  'unknown_server.png',
]

// Colores de texto de las pantallas (los mas usados primero).
const TEXT_COLORS = ['#ffffff', '#a0a0a0', '#808080', '#ff5555', '#55ff55', '#ffaa00', '#707070', '#ffff55', '#e0e0e0', '#ffff00', '#404040']

const held = new Map<string, HTMLImageElement>()

export function holdImage(path: string): Promise<void> {
  if (typeof Image === 'undefined') return Promise.resolve()
  let image = held.get(path)
  if (!image) {
    image = new Image()
    image.decoding = 'async'
    image.src = `/assets/${path}`
    held.set(path, image)
  }
  return image.decode().catch(() => { held.delete(path) })
}

/** Lanza la precarga sin bloquear el arranque. */
export function preloadCore() {
  for (const file of TEXTURES) void holdImage(file)
  preloadSfx()
  void fontReady.then(() => warmAtlases(TEXT_COLORS))
}
