// Tipos que reflejan lo que devuelve el backend (camelCase). Ver docs/IPC.md.

export interface LocalAccountData {
  type: 'local'
  displayName: string
  localUuid: string
  skinId: string | null
}

export interface MicrosoftAccountData {
  type: 'microsoft'
  minecraftUuid: string | null
  minecraftName: string | null
}

export interface AccountRecord {
  id: string
  provider: 'local' | 'microsoft'
  label: string
  createdAt: string
  lastUsedAt: string | null
  providerData: LocalAccountData | MicrosoftAccountData
}

export type JavaSelection = { mode: 'auto' } | { mode: 'path'; path: string }
export type MemoryPolicy =
  | { mode: 'auto' }
  | { mode: 'fixed'; minMb: number; maxMb: number }

export interface Instance {
  id: string
  name: string
  minecraftVersion: string
  loader: 'vanilla' | 'fabric' | 'forge' | 'neoforge' | 'quilt'
  loaderVersion: string | null
  profileId: string
  java: JavaSelection
  memory: MemoryPolicy
  createdAt: string
  updatedAt: string
}

export interface VersionEntry {
  id: string
  type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha'
  releaseTime: string
}

export type InstallPhase =
  | 'metadata'
  | 'client'
  | 'libraries'
  | 'assetIndex'
  | 'assets'
  | 'natives'
  | 'verify'

export interface ProgressEvent {
  instanceId: string
  phase: InstallPhase
  completed: number
  total: number
}

export type LaunchStateEvent = { instanceId: string } & (
  | { state: 'preparing' }
  | { state: 'installing' }
  | { state: 'preparingJava' }
  | { state: 'starting' }
  | { state: 'running'; pid: number }
  | { state: 'exited'; code: number | null }
  | { state: 'failed'; message: string }
)

export interface RepairSummary {
  found: number
  fixed: number
  remaining: number
}

export interface InstanceChanges {
  name?: string
  minecraftVersion?: string
  profileId?: string
  java?: JavaSelection
  memory?: MemoryPolicy
}

export type Theme = 'system' | 'light' | 'dark'
export type DownloadProfile = 'eco' | 'normal' | 'performance'

export interface Settings {
  schemaVersion: number
  theme: Theme
  downloadProfile: DownloadProfile
  javaPath: string | null
}

export interface SettingsSnapshot {
  settings: Settings
  dataDir: string
}
