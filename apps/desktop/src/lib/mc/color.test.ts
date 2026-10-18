import { describe, expect, it } from 'vitest'
import { DEFAULT_HOVER, contrastRatio, hoverFromAction, shadeFromBackground } from './color'

describe('color', () => {
  it('contraste: blanco sobre negro es 21', () => {
    expect(contrastRatio('#ffffff', '#000000')).toBeCloseTo(21, 0)
    expect(contrastRatio('#000000', '#000000')).toBeCloseTo(1, 5)
    expect(contrastRatio('nope', '#000000')).toBe(0)
  })

  it('el tinte conserva un fondo oscuro y oscurece uno claro', () => {
    expect(shadeFromBackground('#04070c')).toBe('4,7,12')
    expect(shadeFromBackground('#ffffff')).toBe('56,56,56')
    expect(shadeFromBackground('basura')).toBe('0,0,0')
  })

  it('el resalte usa la accion del tema solo si contrasta lo suficiente', () => {
    expect(hoverFromAction('#9ad1ff', '0,0,0')).toBe('#9ad1ff')
    expect(hoverFromAction('#222222', '0,0,0')).toBe(DEFAULT_HOVER)
    expect(hoverFromAction('azul', '0,0,0')).toBe(DEFAULT_HOVER)
  })
})
