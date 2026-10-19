import { instances } from './stores/instances.svelte'
import { skins } from './stores/skins.svelte'
import { versions } from './stores/versions.svelte'
import type { View } from './stores/nav.svelte'

let editor: Promise<typeof import('../routes/SkinEditor.svelte')> | null = null
export function loadSkinEditor() {
  return editor ??= import('../routes/SkinEditor.svelte').catch((error) => {
    editor = null
    throw error
  })
}

/** Preparacion anticipada: nunca se espera a ella para cambiar de pantalla. */
export function prepareScreen(view: View) {
  if (view === 'accounts' || view === 'skins') void skins.load()
  if (view === 'create') void versions.prefetch('vanilla')
  if (view === 'edit') void versions.prefetch(instances.selected?.loader ?? 'vanilla')
  if (view === 'skins' || view === 'skinEditor') void loadSkinEditor().catch(() => {})
}

/** Trabajo de baja prioridad despues de mostrar el menu, no en el clic. */
export function warmNavigation(): () => void {
  let cancelled = false
  const run = () => {
    if (cancelled) return
    prepareScreen('accounts')
    prepareScreen('create')
    void versions.prefetch('fabric')
    void loadSkinEditor().catch(() => {})
  }
  if (typeof requestIdleCallback === 'function') {
    const id = requestIdleCallback(run, { timeout: 1500 })
    return () => { cancelled = true; cancelIdleCallback(id) }
  }
  const id = setTimeout(run, 100)
  return () => { cancelled = true; clearTimeout(id) }
}
