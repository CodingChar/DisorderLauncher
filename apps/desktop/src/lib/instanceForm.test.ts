import { describe, expect, it } from 'vitest'
import { formFromInstance, formatGb, javaError, javaSelection, memoryError, memoryPolicy, toGb, PERFORMANCE_LEVELS } from './instanceForm'
import { stringWidth } from './mc/font'
import type { Instance } from './types'

describe('formulario de instancia', () => {
  it('valida la memoria solo si es fija y mantiene el mensaje de siempre', () => {
    expect(memoryError('auto', 999)).toBeNull()
    expect(memoryError('fixed', 4)).toBeNull()
    expect(memoryError('fixed', 0.4)).toBe('Escribe una cantidad de memoria entre 0,5 y 64 GB.')
    expect(memoryError('fixed', 65)).not.toBeNull()
    expect(memoryError('fixed', Number.NaN)).not.toBeNull()
  })

  it('exige la carpeta de Java solo si es propia', () => {
    expect(javaError('auto', '')).toBeNull()
    expect(javaError('path', '  ')).toBe('Indica la carpeta de Java o elige «Automático».')
    expect(javaError('path', 'C:\\Java')).toBeNull()
  })

  it('construye la politica de memoria y la seleccion de Java del contrato', () => {
    expect(memoryPolicy('auto', 8)).toEqual({ mode: 'auto' })
    expect(memoryPolicy('fixed', 4)).toEqual({ mode: 'fixed', minMb: 512, maxMb: 4096 })
    expect(javaSelection('path', ' C:\\Java ')).toEqual({ mode: 'path', path: 'C:\\Java' })
    expect(javaSelection('auto', 'x')).toEqual({ mode: 'auto' })
  })

  it('recupera el estado del formulario desde una instancia', () => {
    const base = { memory: { mode: 'auto' }, java: { mode: 'auto' } } as Instance
    expect(formFromInstance(base)).toEqual({ memoryMode: 'auto', maxGb: 4, javaMode: 'auto', javaPath: '' })
    const custom = { memory: { mode: 'fixed', minMb: 512, maxMb: 6656 }, java: { mode: 'path', path: 'D:\\jdk' } } as Instance
    expect(formFromInstance(custom)).toEqual({ memoryMode: 'fixed', maxGb: 6.5, javaMode: 'path', javaPath: 'D:\\jdk' })
    expect(toGb(2048)).toBe(2)
  })

  it('formatea los GB con coma', () => {
    expect(formatGb(4)).toBe('4 GB')
    expect(formatGb(6.5)).toBe('6,5 GB')
  })

  it('las etiquetas de rendimiento caben en un boton de 150 px junto a «Rendimiento: »', () => {
    for (const level of PERFORMANCE_LEVELS) expect(stringWidth(`Rendimiento: ${level.label}`)).toBeLessThanOrEqual(150 - 8)
  })
})
