// Backend simulado SOLO para desarrollo en navegador (sin Tauri). No se incluye en
// builds de produccion: api/index.ts lo importa de forma dinamica bajo import.meta.env.DEV.

import type { AccountRecord, Instance, ProgressEvent, RepairSummary, VersionEntry } from '../types'

type Handler = (payload: any) => void
const handlers = new Map<string, Set<Handler>>()

export function mockListen(event: string, handler: Handler): () => void {
  const set = handlers.get(event) ?? new Set()
  set.add(handler)
  handlers.set(event, set)
  return () => set.delete(handler)
}

function emit(event: string, payload: unknown) {
  handlers.get(event)?.forEach((h) => h(payload))
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))
let accounts: AccountRecord[] = []
let defaultId: string | null = null
let instances: Instance[] = []
let counter = 0

const versions: VersionEntry[] = ['1.21.4', '1.21.1', '1.20.4', '1.19.4', '1.16.5', '1.12.2'].map((id) => ({
  id,
  type: 'release',
  releaseTime: '2024-01-01T00:00:00+00:00',
}))

async function simulateLaunch(instanceId: string) {
  const state = (s: Record<string, unknown>) => emit('launch://state', { instanceId: instanceId, ...s })
  state({ state: 'preparing' })
  await sleep(400)
  state({ state: 'installing' })
  const phases: ProgressEvent['phase'][] = ['client', 'libraries', 'assets']
  for (const phase of phases) {
    for (let i = 0; i <= 10; i++) {
      emit('install://progress', { instanceId, phase, completed: i * 10, total: 100 })
      await sleep(60)
    }
  }
  state({ state: 'starting' })
  await sleep(500)
  state({ state: 'running', pid: 1234 })
  await sleep(3000)
  state({ state: 'exited', code: 0 })
}

export async function mockCall(cmd: string, args: Record<string, any> = {}): Promise<any> {
  await sleep(80)
  switch (cmd) {
    case 'accounts_list':
      return accounts
    case 'accounts_default':
      return defaultId
    case 'accounts_create_local': {
      const name = String(args.name ?? '').trim()
      if (!/^[A-Za-z0-9_]{3,16}$/.test(name)) {
        throw 'El nombre debe tener entre 3 y 16 caracteres: letras, numeros o guion bajo (_).'
      }
      if (accounts.some((a) => a.label.toLowerCase() === name.toLowerCase())) {
        throw 'Ya existe un perfil con ese nombre.'
      }
      const record: AccountRecord = {
        id: `acc_local_${++counter}`,
        provider: 'local',
        label: name,
        createdAt: new Date().toISOString(),
        lastUsedAt: null,
        providerData: { type: 'local', displayName: name, localUuid: '00000000-0000-3000-8000-000000000000', skinId: null },
      }
      accounts = [...accounts, record]
      defaultId ??= record.id
      return record
    }
    case 'accounts_set_default':
      defaultId = args.id
      return null
    case 'instances_list':
      return instances
    case 'instances_create': {
      if (!defaultId) throw 'Primero crea un perfil para poder jugar.'
      const now = new Date().toISOString()
      const inst: Instance = {
        id: `inst_${++counter}`,
        name: args.name,
        minecraftVersion: args.version,
        loader: 'vanilla',
        loaderVersion: null,
        profileId: defaultId,
        java: { mode: 'auto' },
        memory: { mode: 'auto' },
        createdAt: now,
        updatedAt: now,
      }
      instances = [...instances, inst]
      return inst
    }
    case 'instances_launch':
      void simulateLaunch(args.id)
      return null
    case 'instances_stop':
      emit('launch://state', { instanceId: args.id, state: 'exited', code: null })
      return null
    case 'instances_repair': {
      for (let i = 0; i <= 5; i++) {
        emit('repair://progress', { instanceId: args.id, phase: 'assets', completed: i * 20, total: 100 })
        await sleep(100)
      }
      return { found: 2, fixed: 2, remaining: 0 } satisfies RepairSummary
    }
    case 'minecraft_list_versions':
      return versions
    default:
      throw `Comando desconocido: ${cmd}`
  }
}
