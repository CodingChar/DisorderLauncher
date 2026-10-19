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
  private prepareView: ((view: View) => void) | null = null
  view = $state<View>('menu')
  /** Tipo de instancia que `Crear instancia` preselecciona (lo fijan los atajos de Explorar). */
  createLoader = $state<'vanilla' | 'fabric'>('vanilla')

  go(view: View) {
    this.prepare(view)
    this.view = view
  }

  prepare(view: View) {
    // Un fallo de precarga nunca debe impedir navegar.
    try { this.prepareView?.(view) } catch { /* La pantalla maneja sus propios errores. */ }
  }

  setPreparation(prepare: (view: View) => void): () => void {
    this.prepareView = prepare
    return () => { if (this.prepareView === prepare) this.prepareView = null }
  }

  create(loader: 'vanilla' | 'fabric' = 'vanilla') {
    this.createLoader = loader
    this.go('create')
  }
}

export const nav = new NavStore()
