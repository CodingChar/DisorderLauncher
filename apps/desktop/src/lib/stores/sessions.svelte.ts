import { api, events } from '../api'
import type { ErrorKind, InstallPhase } from '../types'

export type SessionPhase =
  | 'preparing'
  | 'installing'
  | 'preparingJava'
  | 'starting'
  | 'running'
  | 'repairing'
  | 'exited'
  | 'failed'

export interface Session {
  phase: SessionPhase
  message?: string
  kind?: ErrorKind
  exitCode?: number | null
  progress?: { phase: InstallPhase; completed: number; total: number }
  /** Hora local en que se pidio jugar (para mostrarla en el centro de sesion). */
  startedAt?: string
}

export interface LogLine {
  stream: 'stdout' | 'stderr'
  text: string
}

/** Lineas de consola que se conservan por instancia: el juego puede escribir miles. */
const MAX_LOG_LINES = 1000

const BUSY: SessionPhase[] = ['preparing', 'installing', 'preparingJava', 'starting', 'running', 'repairing']

/** Estado de ejecucion por instancia, alimentado por los eventos del backend. */
class SessionsStore {
  byInstance = $state<Record<string, Session>>({})
  /** Salida reciente del juego por instancia (ya sin secretos). */
  logs = $state<Record<string, LogLine[]>>({})
  private started = false
  /** Instancias que el usuario pidio cerrar: su salida no es un fallo. */
  private stopping = new Set<string>()

  get(id: string): Session | undefined {
    return this.byInstance[id]
  }

  logOf(id: string): LogLine[] {
    return this.logs[id] ?? []
  }

  clearLog(id: string) {
    this.logs = { ...this.logs, [id]: [] }
  }

  isBusy(id: string): boolean {
    const s = this.byInstance[id]
    return !!s && BUSY.includes(s.phase)
  }

  async init() {
    if (this.started) return
    this.started = true
    await Promise.all([
      events.onLaunchState((e) => {
        const prev = this.byInstance[e.instanceId]
        switch (e.state) {
          case 'failed':
            this.set(e.instanceId, { phase: 'failed', message: e.message, kind: e.kind })
            break
          case 'exited': {
            const requested = this.stopping.delete(e.instanceId)
            this.set(e.instanceId, { phase: 'exited', exitCode: requested ? 0 : e.code })
            break
          }
          default:
            this.set(e.instanceId, { phase: e.state, progress: e.state === 'installing' ? prev?.progress : undefined })
        }
      }),
      events.onGameOutput((e) => this.appendLog(e.instanceId, { stream: e.stream, text: e.line })),
      events.onInstallProgress((e) => this.progress(e.instanceId, 'installing', e)),
      events.onRepairProgress((e) => this.progress(e.instanceId, 'repairing', e)),
    ])
  }

  async launch(id: string) {
    this.clearLog(id)
    this.set(id, { phase: 'preparing', startedAt: new Date().toLocaleTimeString('es-ES', { hour: '2-digit', minute: '2-digit' }) })
    try {
      await api.instancesLaunch(id)
    } catch (e) {
      this.set(id, { phase: 'failed', message: (e as Error).message, kind: 'other' })
    }
  }

  /** Cierra el aviso de error y devuelve la instancia a su estado normal. */
  dismiss(id: string) {
    const { [id]: _removed, ...rest } = this.byInstance
    this.byInstance = rest
  }

  async stop(id: string) {
    this.stopping.add(id)
    try {
      await api.instancesStop(id)
    } catch (e) {
      this.stopping.delete(id)
      this.set(id, { phase: 'failed', message: (e as Error).message, kind: 'other' })
    }
  }

  async repair(id: string) {
    this.set(id, { phase: 'repairing' })
    try {
      const r = await api.instancesRepair(id)
      this.set(id, {
        phase: 'exited',
        message:
          r.found === 0
            ? 'Todo está en orden. No hacía falta reparar nada.'
            : r.remaining === 0
              ? 'Listo: se repararon los archivos dañados.'
              : 'No se pudo reparar todo. Revisa tu conexión a internet e inténtalo otra vez.',
      })
    } catch (e) {
      this.set(id, { phase: 'failed', message: (e as Error).message, kind: 'repairable' })
    }
  }

  private progress(id: string, phase: SessionPhase, e: { phase: InstallPhase; completed: number; total: number }) {
    const prev = this.byInstance[id]
    if (!prev || prev.phase === 'running') return
    this.set(id, { ...prev, phase, progress: { phase: e.phase, completed: e.completed, total: e.total } })
  }

  private appendLog(id: string, line: LogLine) {
    const current = this.logs[id] ?? []
    const next = current.length >= MAX_LOG_LINES ? [...current.slice(current.length - MAX_LOG_LINES + 1), line] : [...current, line]
    this.logs = { ...this.logs, [id]: next }
  }

  private set(id: string, session: Session) {
    // Los cambios de fase conservan la hora de inicio de la sesion en curso.
    const startedAt = session.startedAt ?? this.byInstance[id]?.startedAt
    this.byInstance = { ...this.byInstance, [id]: { ...session, startedAt } }
  }
}

export const sessions = new SessionsStore()
