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
const installedMods = new Map<string, any[]>()
const pendingUpdates = new Set<string>()
let settings = { schemaVersion: 1, theme: 'system', downloadProfile: 'normal', javaPath: null as string | null }

const versions: VersionEntry[] = ['1.21.4', '1.21.1', '1.20.4', '1.19.4', '1.16.5', '1.12.2'].map((id) => ({
  id,
  type: 'release',
  releaseTime: '2024-01-01T00:00:00+00:00',
}))

async function simulateLaunch(instanceId: string) {
  const state = (s: Record<string, unknown>) => emit('launch://state', { instanceId: instanceId, ...s })
  state({ state: 'preparing' })
  await sleep(400)
  if (instances.find((i) => i.id === instanceId)?.name.toLowerCase().includes('sin red')) {
    state({ state: 'failed', message: 'Necesitas conexion a internet para descargar los archivos que faltan.', kind: 'offline' })
    return
  }
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
        loader: args.loader ?? 'vanilla',
        loaderVersion: args.loader === 'fabric' ? '0.16.9' : null,
        profileId: defaultId,
        java: { mode: 'auto' },
        memory: { mode: 'auto' },
        createdAt: now,
        updatedAt: now,
      }
      instances = [...instances, inst]
      return inst
    }
    case 'instances_update': {
      const inst = instances.find((i) => i.id === args.id)
      if (!inst) throw 'No se encontro la instancia.'
      const c = args.changes ?? {}
      if (c.name !== undefined && !String(c.name).trim()) throw 'el nombre no puede estar vacio'
      if (c.java?.mode === 'path' && !String(c.java.path).includes('jre')) throw 'No se encontro Java en esa carpeta.'
      Object.assign(inst, c, { updatedAt: new Date().toISOString() })
      instances = [...instances]
      return inst
    }
    case 'instances_delete':
      instances = instances.filter((i) => i.id !== args.id)
      return null
    case 'instances_duplicate': {
      const src = instances.find((i) => i.id === args.id)
      if (!src) throw 'No se encontro la instancia.'
      const copy = { ...src, id: `inst_${++counter}`, name: `${src.name} (copia)` }
      instances = [...instances, copy]
      return copy
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
    case 'loaders_supported':
      return ['fabric', 'vanilla']
    case 'mods_list': {
      const inst = instances.find((i) => i.id === args.id)
      const mods = installedMods.get(args.id) ?? []
      const needsApi = mods.some((m) => m.enabled && m.needsApi) && !mods.some((m) => m.enabled && m.name === 'Fabric API')
      return {
        supported: inst?.loader === 'fabric',
        mods: mods.map(({ needsApi: _n, ...m }) => m),
        missingDependencies: needsApi ? [{ modName: mods.find((m) => m.needsApi)!.name, requires: 'Fabric API' }] : [],
      }
    }
    case 'mods_set_enabled': {
      const list = installedMods.get(args.id) ?? []
      const base = (f: string) => f.replace('.disabled', '')
      const m = list.find((x) => base(x.fileName) === base(args.file))
      if (m) {
        m.enabled = args.enabled
        m.fileName = args.enabled ? base(m.fileName) : `${base(m.fileName)}.disabled`
      }
      return null
    }
    case 'mods_remove':
      installedMods.set(args.id, (installedMods.get(args.id) ?? []).filter((m) => m.fileName !== args.file))
      return null
    case 'mods_add_local':
      return null
    case 'mods_search': {
      const all = [
        { project_id: 'AANobbMI', slug: 'sodium', title: 'Sodium', description: 'Motor de renderizado moderno que mejora los FPS.', author: 'jellysquid3', downloads: 50000000, icon_url: null, categories: ['fabric'] },
        { project_id: 'gvQqBUqZ', slug: 'lithium', title: 'Lithium', description: 'Optimiza la logica del juego sin cambiar su comportamiento.', author: 'jellysquid3', downloads: 30000000, icon_url: null, categories: ['fabric'] },
        { project_id: 'P7dR8mSH', slug: 'fabric-api', title: 'Fabric API', description: 'Base que necesitan muchos mods de Fabric.', author: 'modmuss50', downloads: 90000000, icon_url: null, categories: ['fabric'] },
      ]
      const q = String(args.query ?? '').toLowerCase()
      return all.filter((h) => !q || h.title.toLowerCase().includes(q))
    }
    case 'mods_install': {
      const list = installedMods.get(args.id) ?? []
      const result: any[] = []
      const add = (title: string, file: string, dep: boolean, needsApi = false) => {
        list.push({ fileName: file, enabled: true, name: title, version: '1.0.0', description: null, size: 1200000, source: { projectId: file, versionId: 'v1', versionNumber: '1.0.0', hasPreviousVersion: false }, needsApi })
        result.push({ projectId: file, title, versionNumber: '1.0.0', file, isDependency: dep })
      }
      if (args.projectId === 'P7dR8mSH') {
        add('Fabric API', 'fabric-api.jar', false)
      } else {
        if (!list.some((m) => m.name === 'Fabric API') && args.projectId === 'gvQqBUqZ') add('Fabric API', 'fabric-api.jar', true)
        add(args.projectId === 'AANobbMI' ? 'Sodium' : 'Lithium', `${args.projectId}.jar`, false)
        pendingUpdates.add(`${args.projectId}.jar`)
      }
      installedMods.set(args.id, list)
      return result
    }
    case 'mods_check_updates':
      return [...pendingUpdates].map((file) => ({ file, projectId: file, title: file.replace('.jar', ''), currentVersion: '1.0.0', latestVersion: '1.1.0' }))
    case 'mods_update': {
      const m = (installedMods.get(args.id) ?? []).find((x) => x.fileName === args.file)
      if (m) {
        m.version = '1.1.0'
        m.source = { ...m.source, versionNumber: '1.1.0', hasPreviousVersion: true }
      }
      pendingUpdates.delete(args.file)
      return { projectId: args.file, title: m?.name ?? '', versionNumber: '1.1.0', file: args.file, isDependency: false }
    }
    case 'mods_rollback': {
      const m = (installedMods.get(args.id) ?? []).find((x) => x.fileName === args.file)
      if (m) {
        m.version = '1.0.0'
        m.source = { ...m.source, versionNumber: '1.0.0', hasPreviousVersion: false }
      }
      pendingUpdates.add(args.file)
      return null
    }
    case 'settings_get':
      return { settings, dataDir: 'C:\Users\Demo\AppData\Roaming\DisorderLauncher' }
    case 'settings_update':
      settings = { ...args.settings }
      return { settings, dataDir: 'C:\Users\Demo\AppData\Roaming\DisorderLauncher' }
    case 'minecraft_list_versions':
      return versions
    default:
      throw `Comando desconocido: ${cmd}`
  }
}
