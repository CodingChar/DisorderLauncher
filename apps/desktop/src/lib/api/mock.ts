// Backend simulado SOLO para desarrollo en navegador (sin Tauri). No se incluye en
// builds de produccion: api/index.ts lo importa de forma dinamica bajo import.meta.env.DEV.

import type { AccountRecord, Instance, ProgressEvent, RepairSummary, SkinAsset, VersionEntry } from '../types'

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
let mockSkins: { asset: SkinAsset; url: string }[] = []
let msLoginPending = false
const msExpired = new Set<string>()
let mockThemes: any[] = [
  { builtin: true, schemaVersion: 1, id: 'disorderlaucher.default.dark', name: 'Default Dark', mode: 'dark',
    colors: { background: '#0C0C0C', surface: '#141414', surfaceAlt: '#1D1D1D', surfaceRaised: '#252525', textPrimary: '#F0F0EA', textSecondary: '#A0A09A', border: '#2D2D2D', borderStrong: '#666666', action: '#F3F3ED', actionText: '#0C0C0C' },
    typography: { displayFamily: 'Space Grotesk', bodyFamily: 'IBM Plex Sans', scale: 1 },
    shape: { inputRadius: 7, buttonRadius: 8, cardRadius: 13, heroRadius: 18 },
    density: { uiScale: 1, compact: false }, effects: { shadowStrength: 0.2, animations: true } },
  { builtin: true, schemaVersion: 1, id: 'disorderlaucher.default.light', name: 'Default Light', mode: 'light',
    colors: { background: '#ECECE8', surface: '#F8F8F5', surfaceAlt: '#DFDFDB', surfaceRaised: '#CFCFCA', textPrimary: '#0A0A0A', textSecondary: '#55554F', border: '#C8C8C2', borderStrong: '#80807A', action: '#101010', actionText: '#F4F4EF' },
    typography: { displayFamily: 'Space Grotesk', bodyFamily: 'IBM Plex Sans', scale: 1 },
    shape: { inputRadius: 7, buttonRadius: 8, cardRadius: 13, heroRadius: 18 },
    density: { uiScale: 1, compact: false }, effects: { shadowStrength: 0.2, animations: true } },
  { builtin: true, schemaVersion: 1, id: 'disorderlaucher.default.blue', name: 'Azul oscuro', mode: 'dark',
    colors: { background: '#0B1220', surface: '#111D30', surfaceAlt: '#192940', surfaceRaised: '#243954', textPrimary: '#E4EDF9', textSecondary: '#9CADC6', border: '#283C57', borderStrong: '#526E92', action: '#B8D4FA', actionText: '#0B1729' },
    typography: { displayFamily: 'Space Grotesk', bodyFamily: 'IBM Plex Sans', scale: 1 },
    shape: { inputRadius: 7, buttonRadius: 8, cardRadius: 13, heroRadius: 18 },
    density: { uiScale: 1, compact: false }, effects: { shadowStrength: 0.2, animations: true } },
]
const installedMods = new Map<string, any[]>()
const pendingUpdates = new Set<string>()
let settings = { schemaVersion: 1, theme: 'system', downloadProfile: 'normal', themeId: null as string | null, microsoftClientId: null as string | null, javaPath: null as string | null }

const versions: VersionEntry[] = ['1.21.4', '1.21.1', '1.20.4', '1.19.4', '1.16.5', '1.12.2'].map((id) => ({
  id,
  type: 'release',
  releaseTime: '2024-01-01T00:00:00+00:00',
}))

const mockOld: VersionEntry[] = [
  { id: '24w14a', type: 'snapshot', releaseTime: '2024-04-03T00:00:00+00:00' },
  { id: '1.8.9', type: 'release', releaseTime: '2015-12-09T00:00:00+00:00' },
  { id: 'b1.7.3', type: 'old_beta', releaseTime: '2011-07-08T00:00:00+00:00' },
  { id: 'a1.2.6', type: 'old_alpha', releaseTime: '2010-12-03T00:00:00+00:00' },
]

/** Cada tipo de instancia admite versiones distintas (como en el launcher real). */
function mockCatalog(loader: string): VersionEntry[] {
  const all = [...versions, ...mockOld]
  if (loader === 'vanilla') return all
  if (loader === 'neoforge') return all.filter((v) => ['1.21.4', '1.21.1', '1.20.4'].includes(v.id))
  if (loader === 'forge') return all.filter((v) => ['1.20.4', '1.16.5', '1.12.2', '1.8.9'].includes(v.id))
  return all.filter((v) => v.type !== 'old_beta' && v.type !== 'old_alpha' && v.id !== '1.8.9')
}

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
  const out = (stream: 'stdout' | 'stderr', line: string) => emit(`launch://${stream}`, { instanceId, line })
  out('stdout', '[Render thread/INFO]: Setting user: Jugador')
  out('stdout', '[Render thread/INFO]: Backend library: LWJGL version 3.3.3')
  out('stderr', '[Render thread/WARN]: Aviso de ejemplo (solo en la vista previa de desarrollo)')
  await sleep(3000)
  state({ state: 'exited', code: 0 })
}


/** PNG de 64x64 generado en el navegador (solo para la vista previa de desarrollo). */
function makeMockSkin(seed: number): string {
  const palettes = [
    ['#c68e6b', '#3b2a1a', '#2f6bb0', '#2a2f6b'],
    ['#e0ac84', '#d9b44a', '#8a2f5c', '#3a3a4a'],
    ['#a56b4a', '#111111', '#2f8f5b', '#1d3b2a'],
  ]
  const [skin, hair, shirt, pants] = palettes[seed % palettes.length]
  const c = document.createElement('canvas')
  c.width = 64
  c.height = 64
  const g = c.getContext('2d')!
  const r = (col: string, x: number, y: number, w: number, h: number) => {
    g.fillStyle = col
    g.fillRect(x, y, w, h)
  }
  r(skin, 8, 8, 8, 8)
  r(hair, 8, 8, 8, 2)
  r(shirt, 20, 20, 8, 12)
  r(skin, 44, 20, 4, 12)
  r(skin, 36, 52, 4, 12)
  r(pants, 4, 20, 4, 12)
  r(pants, 20, 52, 4, 12)
  r(hair, 40, 8, 8, 3)
  return c.toDataURL('image/png')
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
    case 'microsoft_status':
      return { enabled: !!settings.microsoftClientId, loginInProgress: msLoginPending }
    case 'accounts_microsoft_begin_login': {
      if (!settings.microsoftClientId) throw 'el inicio de sesion con Microsoft no esta configurado en esta version.'
      msLoginPending = true
      emit('auth://state', { state: 'authorizing' })
      void (async () => {
        await sleep(1800)
        if (!msLoginPending) return
        msLoginPending = false
        const record: AccountRecord = {
          id: `acc_ms_${++counter}`,
          provider: 'microsoft',
          label: 'Notch',
          createdAt: new Date().toISOString(),
          lastUsedAt: null,
          providerData: { type: 'microsoft', minecraftUuid: '069a79f4', minecraftName: 'Notch' },
        }
        accounts = [...accounts, record]
        defaultId ??= record.id
        emit('auth://state', { state: 'connected', accountId: record.id, name: 'Notch' })
      })()
      return null
    }
    case 'accounts_microsoft_cancel_login':
      if (msLoginPending) {
        msLoginPending = false
        emit('auth://state', { state: 'cancelled' })
      }
      return null
    case 'accounts_auth_state': {
      const acc = accounts.find((a) => a.id === args.id)
      if (acc?.provider === 'microsoft') return msExpired.has(args.id) ? 'MicrosoftExpired' : 'MicrosoftAuthenticated'
      return 'LocalReady'
    }
    case 'accounts_remove': {
      const using = instances.filter((i) => i.profileId === args.id).length
      if (using > 0) throw `Este perfil lo usan ${using} instancia(s). Cambia el perfil de esas instancias antes de quitarlo.`
      accounts = accounts.filter((a) => a.id !== args.id)
      if (defaultId === args.id) defaultId = accounts[0]?.id ?? null
      return null
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
        loaderVersion: args.loader && args.loader !== 'vanilla' ? '0.16.9' : null,
        profileId: defaultId,
        java: { mode: 'auto' },
        memory: { mode: 'auto' },
        icon: null,
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
      if (c.icon !== undefined) inst.icon = c.icon === '' ? null : c.icon
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
      return ['fabric', 'forge', 'neoforge', 'quilt', 'vanilla']
    case 'mods_list': {
      const inst = instances.find((i) => i.id === args.id)
      const mods = installedMods.get(args.id) ?? []
      const needsApi = mods.some((m) => m.enabled && m.needsApi) && !mods.some((m) => m.enabled && m.name === 'Fabric API')
      return {
        supported: !!inst && inst.loader !== 'vanilla',
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
    case 'modpacks_import': {
      if (!defaultId) throw 'Primero crea un perfil para poder jugar.'
      const id = `inst_${++counter}`
      for (let i = 0; i <= 10; i++) {
        emit('install://progress', { instanceId: id, phase: 'modpack', completed: i * 5, total: 50 })
        await sleep(80)
      }
      const now = new Date().toISOString()
      const inst: Instance = { id, name: 'Pack importado', minecraftVersion: '1.20.1', loader: 'fabric', loaderVersion: '0.15.0', profileId: defaultId, java: { mode: 'auto' }, memory: { mode: 'auto' }, icon: null, createdAt: now, updatedAt: now }
      instances = [...instances, inst]
      return inst
    }
    case 'modpacks_export':
      return { remoteFiles: 12, bundledFiles: 5 }
    case 'skins_list':
      return mockSkins.map((s) => s.asset)
    case 'skins_import': {
      const n = mockSkins.length + 1
      const asset = { id: `skin_${String(n).padStart(32, '0')}`, name: `Skin importada ${n}`, model: (n % 2 ? 'classic' : 'slim') as 'classic' | 'slim', source: 'local' as const, width: 64, height: 64, createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() }
      mockSkins.push({ asset, url: makeMockSkin(n) })
      return asset
    }
    case 'skins_read':
      return mockSkins.find((s) => s.asset.id === args.id)?.url ?? ''
    case 'skins_rename': {
      const s = mockSkins.find((x) => x.asset.id === args.id)
      if (!s) throw 'No se encontro esa skin.'
      s.asset = { ...s.asset, name: args.name }
      return s.asset
    }
    case 'skins_set_model': {
      const s = mockSkins.find((x) => x.asset.id === args.id)
      if (!s) throw 'No se encontro esa skin.'
      s.asset = { ...s.asset, model: args.model }
      return s.asset
    }
    case 'skins_remove':
      mockSkins = mockSkins.filter((s) => s.asset.id !== args.id)
      accounts = accounts.map((a) => (a.providerData.type === 'local' && a.providerData.skinId === args.id ? { ...a, providerData: { ...a.providerData, skinId: null } } : a))
      return null
    case 'skins_save': {
      const url = String(args.pngBase64)
      if (args.id) {
        const s = mockSkins.find((x) => x.asset.id === args.id)
        if (!s) throw 'No se encontro esa skin.'
        s.url = url
        s.asset = { ...s.asset, name: args.name, model: args.model, updatedAt: new Date().toISOString() }
        return s.asset
      }
      const n = mockSkins.length + 1
      const asset = { id: `skin_${String(n).padStart(32, '0')}`, name: args.name, model: args.model, source: 'local' as const, width: 64, height: 64, createdAt: new Date().toISOString(), updatedAt: new Date().toISOString() }
      mockSkins.push({ asset, url })
      return asset
    }
    case 'skins_export':
      return true
    case 'accounts_set_skin': {
      accounts = accounts.map((a) => (a.id === args.accountId && a.providerData.type === 'local' ? { ...a, providerData: { ...a.providerData, skinId: args.skinId } } : a))
      return null
    }
    case 'themes_list':
      return mockThemes
    case 'themes_import': {
      const theme = {
        ...mockThemes[0],
        builtin: false,
        id: `custom.${++counter}`,
        name: 'Medianoche azul',
        colors: { ...mockThemes[0].colors, background: '#0B1020', surface: '#121A30', surfaceAlt: '#1A2442', surfaceRaised: '#243057', border: '#2A3760', borderStrong: '#4A5C94', action: '#8FB4FF', actionText: '#0B1020' },
        shape: { inputRadius: 14, buttonRadius: 20, cardRadius: 26, heroRadius: 30 },
      }
      mockThemes = [...mockThemes, theme]
      return theme
    }
    case 'themes_remove':
      mockThemes = mockThemes.filter((t) => t.id !== args.id)
      if (settings.themeId === args.id) settings = { ...settings, themeId: null }
      return null
    case 'startup_notices':
      return []
    case 'settings_get':
      return { settings, dataDir: 'C:\Users\Demo\AppData\Roaming\DisorderLauncher' }
    case 'settings_update':
      settings = { ...args.settings }
      return { settings, dataDir: 'C:\Users\Demo\AppData\Roaming\DisorderLauncher' }
    case 'minecraft_version_catalog':
      return { versions: mockCatalog(String(args.loader)), source: 'online' }
    case 'minecraft_list_versions':
      return versions
    default:
      throw `Comando desconocido: ${cmd}`
  }
}
