import { nav, type View } from '../stores/nav.svelte'

// Pila de manejadores de «Atras/Esc»: el ultimo registrado (un dialogo, una subpantalla) responde primero.
const stack: (() => void)[] = []

export function pushBack(handler: () => void): () => void {
  stack.push(handler)
  return () => {
    const i = stack.lastIndexOf(handler)
    if (i >= 0) stack.splice(i, 1)
  }
}

/** Ejecuta el manejador mas reciente; `false` si no hay ninguno. */
export function runBack(): boolean {
  const handler = stack.at(-1)
  if (!handler) return false
  handler()
  return true
}

/** A donde lleva «Atras» desde cada pantalla (`navParent` no sirve: es de la barra lateral y varias vistas apuntan a si mismas). */
export const navBack: Record<View, View> = {
  menu: 'menu',
  home: 'instances',
  instances: 'menu',
  explore: 'menu',
  tools: 'menu',
  create: 'instances',
  accounts: 'menu',
  edit: 'instances',
  settings: 'menu',
  mods: 'instances',
  skins: 'accounts',
  skinEditor: 'skins',
}

/** Edicion y mods vuelven a donde se abrieron (Mis instancias o el centro de sesion). */
export function backTarget(view: View, previous: View | null): View {
  if ((view === 'mods' || view === 'edit') && previous === 'home') return 'home'
  return navBack[view]
}

// Vista actual y anterior, para que «Volver» sepa a donde ir.
let current: View | null = null
let previous: View | null = null

/** Se llama en cada cambio de vista (lo hace App). */
export function trackView(view: View) {
  if (view === current) return
  previous = current
  current = view
}

/** «Atras» desde la vista actual (el boton «Volver» de las pantallas y Esc). */
export function goBack() {
  nav.go(backTarget(nav.view, previous))
}
