// Logica pura del selector de version de Minecraft (antes dentro de VersionPicker.svelte).

import type { VersionEntry } from './types'

export type VersionKind = VersionEntry['type']

/** Texto corto para cada tipo de version en la lista. */
export const versionKindLabel: Record<VersionKind, string> = {
  release: 'Estable',
  snapshot: 'Snapshot',
  old_beta: 'Beta',
  old_alpha: 'Alpha',
}

/** Cuanto mostrar de la lista: estables siempre; el resto solo si la persona lo pide (la lista completa es muy larga). */
export type ShowLevel = 'release' | 'snapshot' | 'old_beta' | 'old_alpha'

export const SHOW_LEVELS: { value: ShowLevel; label: string }[] = [
  { value: 'release', label: 'Versiones' },
  { value: 'snapshot', label: '+ Snapshots' },
  { value: 'old_beta', label: '+ Beta' },
  { value: 'old_alpha', label: '+ Alpha' },
]

/** Tipos visibles en cada nivel (cada nivel suma a los anteriores). */
export function showFlags(level: ShowLevel): Record<VersionKind, boolean> {
  const index = SHOW_LEVELS.findIndex((l) => l.value === level)
  return {
    release: true,
    snapshot: index >= 1,
    old_beta: index >= 2,
    old_alpha: index >= 3,
  }
}

/**
 * Versiones a mostrar. La version elegida siempre sigue en la lista (por ejemplo al editar una
 * instancia antigua cuyo tipo esta oculto).
 */
export function filterVersions(list: VersionEntry[], flags: Record<VersionKind, boolean>, current: string): VersionEntry[] {
  const visible = list.filter((v) => flags[v.type])
  if (current && !visible.some((v) => v.id === current)) {
    visible.unshift(list.find((v) => v.id === current) ?? { id: current, type: 'release', releaseTime: '' })
  }
  return visible
}

/** Version a la que pasar cuando la elegida ya no esta en la lista (cambio de tipo de instancia); `null` si no hay que cambiar nada. */
export function pickDefault(list: VersionEntry[], flags: Record<VersionKind, boolean>, current: string): string | null {
  const first = list.find((v) => flags[v.type])?.id
  if (first && (!current || !list.some((v) => v.id === current))) return first
  return null
}

/** «12/03/2021», o cadena vacia si la fecha no es valida. */
export function formatReleaseDate(iso: string): string {
  const d = new Date(iso)
  if (!iso || Number.isNaN(d.getTime())) return ''
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getUTCDate())}/${pad(d.getUTCMonth() + 1)}/${d.getUTCFullYear()}`
}
