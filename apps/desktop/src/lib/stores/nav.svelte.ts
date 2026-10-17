export type View =
  | 'home'
  | 'instances'
  | 'explore'
  | 'tools'
  | 'create'
  | 'accounts'
  | 'edit'
  | 'settings'
  | 'mods'
  | 'skins'
  | 'skinEditor'

/** Etiqueta de cada vista para las migas de pan de la cabecera. */
export const viewLabel: Record<View, string> = {
  home: 'Inicio',
  instances: 'Mis instancias',
  explore: 'Explorar',
  tools: 'Herramientas',
  create: 'Nueva instancia',
  accounts: 'Perfiles',
  edit: 'Configurar instancia',
  settings: 'Ajustes',
  mods: 'Mods',
  skins: 'Skins',
  skinEditor: 'Editor de skins',
}

/** Entrada de la barra lateral que se resalta para cada vista (las secundarias cuelgan de una principal). */
export const navParent: Record<View, View> = {
  home: 'home',
  instances: 'instances',
  explore: 'explore',
  tools: 'tools',
  create: 'instances',
  accounts: 'accounts',
  edit: 'instances',
  settings: 'settings',
  mods: 'instances',
  skins: 'skins',
  skinEditor: 'skins',
}

class NavStore {
  view = $state<View>('home')
  /** Tipo de instancia que `Crear instancia` preselecciona (lo fijan los atajos de Explorar). */
  createLoader = $state<'vanilla' | 'fabric'>('vanilla')

  go(view: View) {
    this.view = view
  }

  create(loader: 'vanilla' | 'fabric' = 'vanilla') {
    this.createLoader = loader
    this.view = 'create'
  }
}

export const nav = new NavStore()
