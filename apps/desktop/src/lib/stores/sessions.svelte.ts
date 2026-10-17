import { api, events } from '../api'
import type { InstallPhase } from '../types'

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
  exitCode?: number | null
  progress?: { phase: InstallPhase; completed: number; total: number }
}

const BUSY: SessionPhase[] = ['preparing', 'installing', 'preparingJava', 'starting', 'running', 'repairing']

/** Estado de ejecucion por instancia, alimentado por los eventos del backend. */
class SessionsStore {
  byInstance = $state<Record<string, Session>>({})
  private started = false

  get(id: string): Session | undefined {
    return this.byInstance[id]
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
            this.set(e.instanceId, { phase: 'failed', message: e.message })
            break
          case 'exited':
            this.set(e.instanceId, { phase: 'exited', exitCode: e.code })
            break
          default:
            this.set(e.instanceId, { phase: e.state, progress: e.state === 'installing' ? prev?.progress : undefined })
        }
      }),
      events.onInstallProgress((e) => this.progress(e.instanceId, 'installing', e)),
      events.onRepairProgress((e) => this.progress(e.instanceId, 'repairing', e)),
    ])
  }

  async launch(id: string) {
    this.set(id, { phase: 'preparing' })
    try {
      await api.instancesLaunch(id)
    } catch (e) {
      this.set(id, { phase: 'failed', message: (e as Error).message })
    }
  }

  async stop(id: string) {
    try {
      await api.instancesStop(id)
    } catch (e) {
      this.set(id, { phase: 'failed', message: (e as Error).message })
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
      this.set(id, { phase: 'failed', message: (e as Error).message })
    }
  }

  private progress(id: string, phase: SessionPhase, e: { phase: InstallPhase; completed: number; total: number }) {
    const prev = this.byInstance[id]
    if (!prev || prev.phase === 'running') return
    this.set(id, { ...prev, phase, progress: { phase: e.phase, completed: e.completed, total: e.total } })
  }

  private set(id: string, session: Session) {
    this.byInstance = { ...this.byInstance, [id]: session }
  }
}

export const sessions = new SessionsStore()
