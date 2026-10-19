import { describe, expect, it } from 'vitest'
import { resolveSound, soundForLabel } from './sfx'

describe('sfx', () => {
  it('reconoce botones de volver sin importar tildes ni mayusculas', () => {
    for (const label of ['Atrás', 'VOLVER', 'Cancelar', 'Salir', 'Hecho', ' No ']) expect(soundForLabel(label)).toBe('back')
  })

  it('el resto de etiquetas suenan a clic', () => {
    for (const label of ['Jugar', 'Cerrar juego', 'Guardar', 'Atrás del todo']) expect(soundForLabel(label)).toBe('click')
  })

  it('respeta la eleccion explicita', () => {
    expect(resolveSound('none', 'Volver')).toBeNull()
    expect(resolveSound('click', 'Volver')).toBe('click')
    expect(resolveSound('auto', 'Volver')).toBe('back')
  })
})
