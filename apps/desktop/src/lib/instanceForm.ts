// Logica pura de los formularios de crear y editar instancia (antes repartida entre PerformanceOptions y las pantallas).

import type { Instance, JavaSelection, LoaderKind, MemoryPolicy, Optimization } from './types'

export const PERFORMANCE_LEVELS: { value: Optimization; label: string; hint: string }[] = [
  { value: 'off', label: 'Normal', hint: 'Sin cambios. Para equipos con buen rendimiento.' },
  { value: 'balanced', label: 'Equilibrado', hint: 'Menos tirones: distancia de visión moderada y menos partículas.' },
  { value: 'performance', label: 'Alto rendimiento', hint: 'Equipo potente (6 GB de RAM o más): mejor gestión de memoria para modpacks grandes.' },
  { value: 'lowEnd', label: 'Modesto', hint: 'Equipo modesto: lo mínimo en gráficos y menos carga para el procesador y la memoria.' },
]

export const LOADER_OPTIONS: { value: LoaderKind; label: string }[] = [
  { value: 'vanilla', label: 'Original' },
  { value: 'fabric', label: 'Fabric' },
  { value: 'quilt', label: 'Quilt' },
  { value: 'forge', label: 'Forge' },
  { value: 'neoforge', label: 'NeoForge' },
]

export const MEMORY_MODES = [
  { value: 'auto' as const, label: 'Automática' },
  { value: 'fixed' as const, label: 'Elegir cantidad' },
]

export const JAVA_MODES = [
  { value: 'auto' as const, label: 'Automático' },
  { value: 'path' as const, label: 'Mi propio Java' },
]

export const MEMORY_HINT = 'Más memoria no siempre es mejor. 4 GB suele bastar.'
export const MIN_GB = 0.5
export const MAX_GB = 64

/** Error de memoria listo para mostrar, o `null` si es valida (solo importa con memoria fija). */
export function memoryError(mode: 'auto' | 'fixed', gb: number): string | null {
  if (mode === 'fixed' && (!Number.isFinite(gb) || gb < MIN_GB || gb > MAX_GB)) return 'Escribe una cantidad de memoria entre 0,5 y 64 GB.'
  return null
}

export function javaError(mode: 'auto' | 'path', path: string): string | null {
  if (mode === 'path' && path.trim().length === 0) return 'Indica la carpeta de Java o elige «Automático».'
  return null
}

export const memoryPolicy = (mode: 'auto' | 'fixed', gb: number): MemoryPolicy =>
  mode === 'auto' ? { mode: 'auto' } : { mode: 'fixed', minMb: 512, maxMb: Math.round(gb * 1024) }

export const javaSelection = (mode: 'auto' | 'path', path: string): JavaSelection => (mode === 'auto' ? { mode: 'auto' } : { mode: 'path', path: path.trim() })

/** GB con un decimal (lo que se muestra y se vuelve a guardar). */
export const toGb = (maxMb: number) => Math.round((maxMb / 1024) * 10) / 10

/** Estado inicial del formulario a partir de una instancia existente. */
export function formFromInstance(instance: Instance) {
  return {
    memoryMode: instance.memory.mode === 'fixed' ? ('fixed' as const) : ('auto' as const),
    maxGb: instance.memory.mode === 'fixed' ? toGb(instance.memory.maxMb) : 4,
    javaMode: instance.java.mode === 'path' ? ('path' as const) : ('auto' as const),
    javaPath: instance.java.mode === 'path' ? instance.java.path : '',
  }
}

/** Texto de GB para el deslizador («4 GB», «6,5 GB»). */
export const formatGb = (gb: number) => `${String(gb).replace('.', ',')} GB`
