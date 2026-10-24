// Efectos de interfaz. Un solo punto de entrada: `playSfx('click')`.
// Web Audio con buffers ya decodificados: sin latencia de <audio>, solapables y sin crear elementos por clic.
// Para anadir un efecto: copia el mp3 a `public/assets/sounds/` y registralo en SOUNDS.

export const SOUNDS = {
  /** Pulsar cualquier boton, pestana, fila o casilla. */
  click: { file: 'minecraft_click.mp3', gain: 1 },
  /** Volver, cancelar, cerrar un dialogo o salir. */
  back: { file: 'back-or-exit.mp3', gain: 1 },
  /** Pasar el puntero o moverse con teclado por un boton (Minecraft Legacy). */
  focus: { file: 'legacy-focus.mp3', gain: 0.6, gap: 60 },
  /** Mover un deslizador o la seleccion de una lista (Minecraft Legacy). */
  scroll: { file: 'legacy-scroll.mp3', gain: 0.8, gap: 70 },
  /** Dialogo de error o algo que no se pudo hacer (Minecraft Legacy). */
  fail: { file: 'legacy-fail.mp3', gain: 1 },
} as const satisfies Record<string, { file: string; gain: number; gap?: number }>

export type SoundId = keyof typeof SOUNDS
export type SoundChoice = SoundId | 'auto' | 'none'

/** Etiquetas de botones que significan «volver» (coincidencia exacta, sin distinguir tildes ni mayusculas). */
const BACK_LABELS = new Set(['atras', 'volver', 'cancelar', 'salir', 'cerrar', 'hecho', 'no'])

const normalize = (text: string) => text.normalize('NFD').replace(/[̀-ͯ]/g, '').trim().toLowerCase()

/** Sonido de un boton `auto`: «Atras», «Volver», «Cancelar»... suenan a volver; el resto, a clic. */
export function soundForLabel(label: string): SoundId {
  return BACK_LABELS.has(normalize(label)) ? 'back' : 'click'
}

export function resolveSound(choice: SoundChoice, label: string): SoundId | null {
  return choice === 'none' ? null : choice === 'auto' ? soundForLabel(label) : choice
}

// Evita que un doble disparo del mismo evento (clic + Enter, etc.) sature el sonido.
const MIN_GAP_MS = 25
const STORAGE_KEY = 'disorder.sfxVolume'

let context: AudioContext | null = null
let master: GainNode | null = null
const buffers = new Map<SoundId, AudioBuffer>()
const loading = new Map<SoundId, Promise<void>>()
const lastPlayed = new Map<SoundId, number>()
let volume = readVolume()

function readVolume(): number {
  try {
    const stored = Number(localStorage.getItem(STORAGE_KEY))
    return localStorage.getItem(STORAGE_KEY) !== null && Number.isFinite(stored) ? Math.max(0, Math.min(1, stored)) : 1
  } catch {
    return 1
  }
}

function ensureContext(): AudioContext | null {
  if (context) return context
  const Ctor = typeof window === 'undefined' ? undefined : window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
  if (!Ctor) return null
  try {
    context = new Ctor()
    master = context.createGain()
    master.gain.value = volume
    master.connect(context.destination)
  } catch {
    context = null
  }
  return context
}

function load(id: SoundId): Promise<void> {
  const existing = loading.get(id)
  if (existing) return existing
  const ctx = ensureContext()
  if (!ctx) return Promise.resolve()
  const task = fetch(`/assets/sounds/${SOUNDS[id].file}`)
    .then((response) => response.arrayBuffer())
    .then((data) => ctx.decodeAudioData(data))
    .then((buffer) => void buffers.set(id, buffer))
    .catch(() => void loading.delete(id))
  loading.set(id, task)
  return task
}

/** Descarga y decodifica los efectos en segundo plano (llamar al arrancar). */
export function preloadSfx(ids: readonly SoundId[] = Object.keys(SOUNDS) as SoundId[]) {
  for (const id of ids) void load(id)
}

export function playSfx(id: SoundId | null) {
  if (!id || volume === 0) return
  const ctx = ensureContext()
  if (!ctx || !master) return
  const now = performance.now()
  const gap = (SOUNDS[id] as { gap?: number }).gap ?? MIN_GAP_MS
  if (now - (lastPlayed.get(id) ?? -Infinity) < gap) return
  lastPlayed.set(id, now)
  // Navegadores que arrancan el contexto suspendido hasta el primer gesto del usuario.
  if (ctx.state === 'suspended') void ctx.resume().catch(() => {})
  const buffer = buffers.get(id)
  if (!buffer) {
    void load(id)
    return
  }
  const source = ctx.createBufferSource()
  source.buffer = buffer
  const gain = SOUNDS[id].gain
  if (gain === 1) {
    source.connect(master)
  } else {
    const level = ctx.createGain()
    level.gain.value = gain
    source.connect(level).connect(master)
  }
  source.start()
}

export const sfxVolume = () => volume

/** Volumen general de los efectos (0 a 1); se recuerda entre sesiones. */
export function setSfxVolume(next: number) {
  volume = Math.max(0, Math.min(1, next))
  if (master) master.gain.value = volume
  try {
    localStorage.setItem(STORAGE_KEY, String(volume))
  } catch { /* Sin almacenamiento: el volumen vale solo esta sesion. */ }
}
