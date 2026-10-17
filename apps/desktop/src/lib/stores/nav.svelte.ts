export type View = 'home' | 'create' | 'accounts' | 'edit'

class NavStore {
  view = $state<View>('home')

  go(view: View) {
    this.view = view
  }
}

export const nav = new NavStore()
