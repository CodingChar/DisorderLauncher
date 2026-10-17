import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  AccountRecord,
  Instance,
  LaunchStateEvent,
  ProgressEvent,
  RepairSummary,
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
  accountsSetDefault: (id: string) => call<void>('accounts_set_default', { id }),
  instancesList: () => call<Instance[]>('instances_list'),
  instancesCreate: (name: string, version: string) => call<Instance>('instances_create', { name, version }),
  instancesLaunch: (id: string) => call<void>('instances_launch', { id }),
  instancesStop: (id: string) => call<void>('instances_stop', { id }),
  instancesRepair: (id: string) => call<RepairSummary>('instances_repair', { id }),
  listVersions: (includeAll = false) => call<VersionEntry[]>('minecraft_list_versions', { includeAll }),
}

export const events = {
  onLaunchState: (h: (e: LaunchStateEvent) => void) => subscribe('launch://state', h),
  onInstallProgress: (h: (e: ProgressEvent) => void) => subscribe('install://progress', h),
  onRepairProgress: (h: (e: ProgressEvent) => void) => subscribe('repair://progress', h),
}
