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

export type LoaderKind = 'vanilla' | 'fabric' | 'forge' | 'neoforge' | 'quilt'

export type JavaSelection = { mode: 'auto' } | { mode: 'path'; path: string }
export type MemoryPolicy =
  | { mode: 'auto' }
  | { mode: 'fixed'; minMb: number; maxMb: number }

export interface Instance {
  id: string
  name: string
  minecraftVersion: string
  loader: LoaderKind
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

/** Que accion ofrecer ante un fallo: reintentar con internet, reparar archivos u otra cosa. */
export type ErrorKind = 'offline' | 'repairable' | 'other'

export type InstallPhase =
  | 'metadata'
  | 'loader'
  | 'modpack'
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
  | { state: 'failed'; message: string; kind: ErrorKind }
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
  themeId: string | null
  javaPath: string | null
}

export interface SettingsSnapshot {
  settings: Settings
  dataDir: string
}

export interface ModSource {
  projectId: string
  versionId: string
  versionNumber: string
  hasPreviousVersion: boolean
}

export interface ModEntry {
  fileName: string
  enabled: boolean
  name: string
  version: string | null
  description: string | null
  size: number
  source: ModSource | null
}

export interface MissingDependency {
  modName: string
  requires: string
}

export interface ModsOverview {
  mods: ModEntry[]
  missingDependencies: MissingDependency[]
  supported: boolean
}

export interface ProjectHit {
  project_id: string
  slug: string
  title: string
  description: string
  author: string
  downloads: number
  icon_url: string | null
  categories: string[]
}

export interface InstalledMod {
  projectId: string
  title: string
  versionNumber: string
  file: string
  isDependency: boolean
}

export interface UpdateInfo {
  file: string
  projectId: string
  title: string
  currentVersion: string
  latestVersion: string
}

export interface LoaderVersion {
  version: string
  stable: boolean
}

export interface ExportSummary {
  remoteFiles: number
  bundledFiles: number
}

export interface ThemeColors {
  background: string
  surface: string
  surfaceAlt: string
  surfaceRaised: string
  textPrimary: string
  textSecondary: string
  border: string
  borderStrong: string
  action: string
  actionText: string
}

export interface ThemeEntry {
  builtin: boolean
  schemaVersion: number
  id: string
  name: string
  mode: 'dark' | 'light'
  colors: ThemeColors
  typography: { displayFamily: string; bodyFamily: string; scale: number }
  shape: { inputRadius: number; buttonRadius: number; cardRadius: number; heroRadius: number }
  density: { uiScale: number; compact: boolean }
  effects: { shadowStrength: number; animations: boolean }
}
