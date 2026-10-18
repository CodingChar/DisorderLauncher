export type View =
  | 'menu'
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

/** Nombre de cada vista (titulo accesible de la pantalla). */
export const viewLabel: Record<View, string> = {
  menu: 'Menú principal',
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

/** Pantalla principal a la que pertenece cada vista (las secundarias cuelgan de una principal). */
export const navParent: Record<View, View> = {
  menu: 'menu',
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
  view = $state<View>('menu')
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
