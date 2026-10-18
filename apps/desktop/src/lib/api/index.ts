import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  AccountRecord,
  AuthEvent,
  AuthState,
  MicrosoftStatus,
  ExportSummary,
  Instance,
  InstanceChanges,
  InstalledMod,
  LoaderKind,
  ModEntry,
  ModsOverview,
  ProjectHit,
  UpdateInfo,
  LaunchStateEvent,
  GameOutputEvent,
  ProgressEvent,
  RepairSummary,
  SkinAsset,
  SkinModel,
  Settings,
  SettingsSnapshot,
  ThemeEntry,
  VersionCatalog,
  VersionEntry,
} from '../types'

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const GENERIC_ERROR = 'Ocurrió un error inesperado. Inténtalo de nuevo.'

type MockModule = typeof import('./mock')
let mockModule: Promise<MockModule> | null = null

// En desarrollo, fuera de Tauri, se usa un backend simulado para poder revisar la UI en el navegador.
function mock(): Promise<MockModule> | null {
  if (inTauri || !import.meta.env.DEV) return null
  mockModule ??= import('./mock')
  return mockModule
}

/** El backend ya devuelve mensajes pensados para el usuario; aqui solo se normalizan. */
function toMessage(error: unknown): string {
  if (typeof error === 'string' && error.trim()) return error
  if (error instanceof Error && error.message) return error.message
  return GENERIC_ERROR
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const m = mock()
    if (m) return (await (await m).mockCall(command, args)) as T
    return await invoke<T>(command, args)
  } catch (error) {
    throw new Error(toMessage(error))
  }
}

async function subscribe<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
  const m = mock()
  if (m) return (await m).mockListen(event, handler)
  return listen<T>(event, (e) => handler(e.payload))
}

export const api = {
  accountsList: () => call<AccountRecord[]>('accounts_list'),
  accountsDefault: () => call<string | null>('accounts_default'),
  accountsCreateLocal: (name: string) => call<AccountRecord>('accounts_create_local', { name }),
  microsoftStatus: () => call<MicrosoftStatus>('microsoft_status'),
  accountsMicrosoftBeginLogin: () => call<void>('accounts_microsoft_begin_login'),
  accountsMicrosoftCancelLogin: () => call<void>('accounts_microsoft_cancel_login'),
  accountsAuthState: (id: string) => call<AuthState>('accounts_auth_state', { id }),
  accountsRemove: (id: string) => call<void>('accounts_remove', { id }),
  accountsSetDefault: (id: string) => call<void>('accounts_set_default', { id }),
  instancesList: () => call<Instance[]>('instances_list'),
  instancesCreate: (name: string, version: string, loader: LoaderKind = 'vanilla') =>
    call<Instance>('instances_create', { name, version, loader }),
  loadersSupported: () => call<LoaderKind[]>('loaders_supported'),
  instancesUpdate: (id: string, changes: InstanceChanges) => call<Instance>('instances_update', { id, changes }),
  instancesDelete: (id: string) => call<void>('instances_delete', { id }),
  instancesDuplicate: (id: string) => call<Instance>('instances_duplicate', { id }),
  instancesLaunch: (id: string) => call<void>('instances_launch', { id }),
  instancesStop: (id: string) => call<void>('instances_stop', { id }),
  instancesRepair: (id: string) => call<RepairSummary>('instances_repair', { id }),
  modsList: (id: string) => call<ModsOverview>('mods_list', { id }),
  modsSetEnabled: (id: string, file: string, enabled: boolean) => call<void>('mods_set_enabled', { id, file, enabled }),
  modsRemove: (id: string, file: string) => call<void>('mods_remove', { id, file }),
  modsAddLocal: (id: string) => call<ModEntry | null>('mods_add_local', { id }),
  modsSearch: (id: string, query: string, page = 0) => call<ProjectHit[]>('mods_search', { id, query, page }),
  modsInstall: (id: string, projectId: string) => call<InstalledMod[]>('mods_install', { id, projectId }),
  modsCheckUpdates: (id: string) => call<UpdateInfo[]>('mods_check_updates', { id }),
  modsUpdate: (id: string, file: string) => call<InstalledMod>('mods_update', { id, file }),
  modsRollback: (id: string, file: string) => call<void>('mods_rollback', { id, file }),
  modpacksImport: () => call<Instance | null>('modpacks_import'),
  modpacksExport: (id: string) => call<ExportSummary | null>('modpacks_export', { id }),
  skinsList: () => call<SkinAsset[]>('skins_list'),
  skinsImport: () => call<SkinAsset | null>('skins_import'),
  skinsRead: (id: string) => call<string>('skins_read', { id }),
  skinsRename: (id: string, name: string) => call<SkinAsset>('skins_rename', { id, name }),
  skinsSetModel: (id: string, model: SkinModel) => call<SkinAsset>('skins_set_model', { id, model }),
  skinsRemove: (id: string) => call<void>('skins_remove', { id }),
  skinsSave: (id: string | null, name: string, pngBase64: string, model: SkinModel) =>
    call<SkinAsset>('skins_save', { id, name, pngBase64, model }),
  skinsExport: (id: string) => call<boolean>('skins_export', { id }),
  accountsSetSkin: (accountId: string, skinId: string | null) => call<void>('accounts_set_skin', { accountId, skinId }),
  themesList: () => call<ThemeEntry[]>('themes_list'),
  themesImport: () => call<ThemeEntry | null>('themes_import'),
  themesRemove: (id: string) => call<void>('themes_remove', { id }),
  startupNotices: () => call<string[]>('startup_notices'),
  settingsGet: () => call<SettingsSnapshot>('settings_get'),
  settingsUpdate: (settings: Settings) => call<SettingsSnapshot>('settings_update', { settings }),
  versionCatalog: (loader: LoaderKind) => call<VersionCatalog>('minecraft_version_catalog', { loader }),
  listVersions: (includeAll = false) => call<VersionEntry[]>('minecraft_list_versions', { includeAll }),
}

export const events = {
  onLaunchState: (h: (e: LaunchStateEvent) => void) => subscribe('launch://state', h),
  onAuthState: (h: (e: AuthEvent) => void) => subscribe('auth://state', h),
  onInstallProgress: (h: (e: ProgressEvent) => void) => subscribe('install://progress', h),
  /** Salida del juego (stdout y stderr, ya con secretos redactados por el backend). */
  onGameOutput: async (h: (e: GameOutputEvent) => void) => {
    const stops = await Promise.all([
      subscribe<{ instanceId: string; line: string }>('launch://stdout', (e) => h({ ...e, stream: 'stdout' })),
      subscribe<{ instanceId: string; line: string }>('launch://stderr', (e) => h({ ...e, stream: 'stderr' })),
    ])
    return () => stops.forEach((stop) => stop())
  },
  onRepairProgress: (h: (e: ProgressEvent) => void) => subscribe('repair://progress', h),
}
