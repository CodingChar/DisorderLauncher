import { fontReady } from './font'

/** `ready` pasa a true cuando la textura de la fuente esta lista: los textos se vuelven a dibujar. */
export const fontStatus = $state({ ready: false })

fontReady.then(() => {
  fontStatus.ready = true
})
