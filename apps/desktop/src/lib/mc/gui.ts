import { getContext, setContext } from 'svelte'

/** Medidas del escenario en px de GUI. `McStage` lo mantiene al dia; las pantallas lo leen para colocarse. */
export interface Gui {
  /** Ancho virtual. */
  W: number
  /** Alto virtual. */
  H: number
  /** Centro horizontal (`floor(W / 2)`): el origen de las `x`. */
  cx: number
  /** Px de pantalla por px de GUI (puede ser fraccionario con DPI no entero). */
  s: number
  /** Factor entero de GUI en px de dispositivo (para dibujar a resolucion nativa). */
  factor: number
}

const KEY = Symbol('mc-gui')

export const provideGui = (gui: Gui) => setContext(KEY, gui)
export const useGui = (): Gui => getContext<Gui>(KEY)
