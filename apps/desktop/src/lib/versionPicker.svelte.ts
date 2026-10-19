import { filterVersions, pickDefault, showFlags, type ShowLevel } from './versionPicker'
import { versions } from './stores/versions.svelte'
import type { LoaderKind } from './types'
import { untrack } from 'svelte'

/**
 * Estado del selector de version de Minecraft para una pantalla. Debe crearse al inicializar el componente
 * (registra efectos). Cada cambio de tipo de instancia recarga la lista: cada loader soporta versiones distintas.
 */
export function useVersionPicker(options: {
  loader: () => LoaderKind
  value: () => string
  setValue: (value: string) => void
  /** Al cambiar de tipo de instancia, pasar a una version valida (no al editar una existente). */
  autoSelect?: boolean
}) {
  const autoSelect = options.autoSelect ?? true
  let level = $state<ShowLevel>('release')

  $effect(() => {
    const loader = options.loader()
    untrack(() => { void versions.load(loader) })
  })

  const flags = $derived(showFlags(level))
  const entries = $derived(filterVersions(versions.list, flags, options.value()))

  // Si la version elegida ya no esta (cambio de tipo de instancia), se pasa a la primera disponible.
  $effect(() => {
    if (!autoSelect || versions.loading || versions.loader !== options.loader()) return
    const next = pickDefault(versions.list, flags, options.value())
    if (next) options.setValue(next)
  })

  return {
    get level() {
      return level
    },
    set level(next: ShowLevel) {
      level = next
    },
    get entries() {
      return entries
    },
    reload: () => versions.load(options.loader(), true),
  }
}
