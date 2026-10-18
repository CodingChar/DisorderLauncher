import { describe, expect, it } from 'vitest'
import { fitText, normalizeText, stringWidth, tailText, wrap } from './font'

describe('stringWidth', () => {
  it('suma los anchos de FontRenderer', () => {
    expect(stringWidth('')).toBe(0)
    expect(stringWidth('i')).toBe(2)
    expect(stringWidth(' ')).toBe(4)
    expect(stringWidth('Aa')).toBe(12)
  })

  it('mide los acentos del español y un caracter sin glifo como «?»', () => {
    expect(stringWidth('ñ')).toBe(6)
    expect(stringWidth('¿')).toBe(6)
    expect(stringWidth('😀')).toBe(stringWidth('?'))
  })

  it('tolera null y undefined', () => {
    expect(stringWidth(undefined)).toBe(0)
    expect(stringWidth(null)).toBe(0)
  })
})

describe('fitText', () => {
  it('no toca lo que cabe', () => {
    expect(fitText('Hola', 100)).toBe('Hola')
    expect(fitText('Hola', 0)).toBe('Hola')
  })

  it('acorta con puntos suspensivos sin pasarse del ancho', () => {
    const out = fitText('Un nombre de instancia demasiado largo', 80)
    expect(out.endsWith('...')).toBe(true)
    expect(stringWidth(out)).toBeLessThanOrEqual(80)
    expect(out.length).toBeGreaterThan(4)
  })

  it('conserva al menos un caracter y es rapido con 2000 caracteres', () => {
    expect(fitText('xxxxxxxxxx', 4)).toBe('x...')
    const started = performance.now()
    const out = fitText('a'.repeat(2000), 300)
    expect(performance.now() - started).toBeLessThan(50)
    expect(stringWidth(out)).toBeLessThanOrEqual(300)
  })
})

describe('tailText', () => {
  it('se queda con el final', () => {
    const out = tailText('abcdefghijklmnopqrstuvwxyz', 60)
    expect(stringWidth(out)).toBeLessThanOrEqual(60)
    expect('abcdefghijklmnopqrstuvwxyz'.endsWith(out)).toBe(true)
  })

  it('devuelve al menos el ultimo caracter y cadena vacia si no hay nada', () => {
    expect(tailText('abc', 1)).toBe('c')
    expect(tailText('', 10)).toBe('')
  })
})

describe('wrap', () => {
  it('parte por palabras', () => {
    expect(wrap('a b c', 1000)).toEqual(['a b c'])
    const lines = wrap('uno dos tres cuatro cinco seis', 50)
    expect(lines.length).toBeGreaterThan(1)
    for (const l of lines) expect(stringWidth(l)).toBeLessThanOrEqual(50)
    expect(lines.join(' ')).toBe('uno dos tres cuatro cinco seis')
  })

  it('corta por caracter una palabra mas larga que el ancho', () => {
    const lines = wrap('x'.repeat(60), 30)
    expect(lines.length).toBeGreaterThan(1)
    for (const l of lines) expect(stringWidth(l)).toBeLessThanOrEqual(30)
    expect(lines.join('')).toBe('x'.repeat(60))
  })

  it('respeta los saltos de linea y la cadena vacia', () => {
    expect(wrap('a\nb', 100)).toEqual(['a', 'b'])
    expect(wrap('', 100)).toEqual([''])
    expect(wrap(undefined, 100)).toEqual([''])
  })

  it('con maxLines acorta la ultima linea con puntos suspensivos', () => {
    const lines = wrap('palabra '.repeat(40), 80, 2)
    expect(lines).toHaveLength(2)
    expect(lines[1].endsWith('...')).toBe(true)
    expect(stringWidth(lines[1])).toBeLessThanOrEqual(80)
  })

  it('no añade puntos si el texto cabe en maxLines', () => {
    expect(wrap('hola mundo', 200, 2)).toEqual(['hola mundo'])
  })
})

describe('signos sin glifo', () => {
  it('sustituye … · y rayas por equivalentes de la fuente', () => {
    expect(normalizeText('Guardando… 1 · 2 — 3')).toBe('Guardando... 1 - 2 - 3')
    expect(stringWidth('Hola…')).toBe(stringWidth('Hola...'))
  })

  it('las vocales acentuadas que la textura no tiene usan la letra base', () => {
    expect(stringWidth('Á')).toBe(stringWidth('A'))
    expect(stringWidth('Ú')).toBe(stringWidth('U'))
    expect(stringWidth('é')).toBe(stringWidth('é'))
    expect(stringWidth('Ж')).toBe(stringWidth('?'))
  })

  it('fitText sigue respetando el ancho tras sustituir', () => {
    expect(stringWidth(fitText('Un momento… un momento… un momento…', 90))).toBeLessThanOrEqual(90)
  })
})
