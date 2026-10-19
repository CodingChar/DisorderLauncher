import { afterEach, describe, expect, it } from 'vitest'
import { backTarget, navBack, pushBack, runBack } from './back'
import { navParent, viewLabel, type View } from '../stores/nav.svelte'

const VIEWS = Object.keys(navBack) as View[]

describe('navegacion', () => {
  it('todas las vistas tienen etiqueta, entrada principal y destino de «Atras»', () => {
    expect(Object.keys(viewLabel).sort()).toEqual([...VIEWS].sort())
    expect(Object.keys(navParent).sort()).toEqual([...VIEWS].sort())
    expect(viewLabel.menu).toBeTruthy()
    expect(navParent.menu).toBe('menu')
  })

  it('«Atras» nunca lleva a la misma vista salvo en el menu', () => {
    for (const view of VIEWS) if (view !== 'menu') expect(navBack[view]).not.toBe(view)
  })

  it('editar y mods vuelven al centro de sesion si vienen de el', () => {
    expect(backTarget('edit', 'home')).toBe('home')
    expect(backTarget('mods', 'home')).toBe('home')
    expect(backTarget('edit', 'instances')).toBe('instances')
    expect(backTarget('instances', 'home')).toBe('menu')
    expect(backTarget('skinEditor', null)).toBe('skins')
    expect(backTarget('skins', 'skinEditor')).toBe('menu')
  })
})

describe('pila de «Atras»', () => {
  const offs: (() => void)[] = []
  afterEach(() => void offs.splice(0).forEach((off) => off()))

  it('responde primero el ultimo manejador y al quitarlo vuelve el anterior', () => {
    const calls: string[] = []
    offs.push(pushBack(() => calls.push('pantalla')))
    const off = pushBack(() => calls.push('dialogo'))
    offs.push(off)
    expect(runBack()).toBe(true)
    off()
    expect(runBack()).toBe(true)
    expect(calls).toEqual(['dialogo', 'pantalla'])
  })

  it('sin manejadores devuelve false', () => {
    expect(runBack()).toBe(false)
  })
})
