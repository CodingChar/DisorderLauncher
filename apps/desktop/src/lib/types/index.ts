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

/** Nivel de optimizacion de una instancia para equipos modestos. */
export type Optimization = 'off' | 'balanced' | 'performance' | 'lowEnd'

export type JvmRisk = 'safe' | 'tradeoff' | 'experimental'

/** Un ajuste de Java del catalogo del launcher, tal como lo describe `instances_jvm_check`. */
export interface JvmTweakInfo {
  id: string
  label: string
  hint: string
  risk: JvmRisk
  source: string
  selected: boolean
  /** Ya lo aplica el nivel de rendimiento elegido. */
  fromLevel: boolean
  /** Motivo por el que no sirve en este Java o equipo; `null`: disponible. */
  unavailable: string | null
}

export interface JvmCheck {
  javaMajor: number
  /** `true`: probado con el Java instalado. `false`: ese Java aun no esta descargado. */
  verified: boolean
  tweaks: JvmTweakInfo[]
  flags: string[]
  skipped: { id: string; reason: string }[]
}

export interface OptimizationReport {
  installed: string[]
  skipped: { name: string; reason: string }[]
}

export interface Instance {
  id: string
  name: string
  minecraftVersion: string
  loader: LoaderKind
  loaderVersion: string | null
  profileId: string
  java: JavaSelection
  memory: MemoryPolicy
  /** Icono elegido (id de `itemIcons`); `null`: el del tipo de instancia. */
  icon: string | null
  optimization: Optimization
  /** Ajustes de Java activados a mano (ids del catalogo). */
  jvmTweaks: string[]
  /** Argumentos de Java propios, separados por espacios. */
  jvmArgs: string
  createdAt: string
  updatedAt: string
}

export interface VersionCatalog {
  versions: VersionEntry[]
  /** `online`: recien descargada; `cached`: ultima descargada; `installed`: solo lo ya instalado. */
  source: 'online' | 'cached' | 'installed'
}

export interface VersionEntry {
  id: string
  type: 'release' | 'snapshot' | 'old_beta' | 'old_alpha'
  releaseTime: string
}

/** Que accion ofrecer ante un fallo: reintentar con internet, reparar archivos u otra cosa. */
export type ErrorKind = 'offline' | 'repairable' | 'signIn' | 'other'

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

export interface GameOutputEvent {
  instanceId: string
  line: string
  stream: 'stdout' | 'stderr'
}

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
  /** Cadena vacia: vuelve al icono por defecto. */
  icon?: string
  optimization?: Optimization
  jvmTweaks?: string[]
  jvmArgs?: string
}

export type Theme = 'system' | 'light' | 'dark'
export type DownloadProfile = 'eco' | 'normal' | 'performance'

export interface Settings {
  schemaVersion: number
  theme: Theme
  downloadProfile: DownloadProfile
  themeId: string | null
  microsoftClientId: string | null
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

export type AuthState =
  | 'LocalReady'
  | 'MicrosoftDisconnected'
  | 'MicrosoftAuthorizing'
  | 'MicrosoftAuthenticated'
  | 'MicrosoftRefreshRequired'
  | 'MicrosoftOfflineCachedIdentity'
  | 'MicrosoftExpired'
  | { Error: unknown }

export interface MicrosoftStatus {
  enabled: boolean
  loginInProgress: boolean
}

export type AuthEvent =
  | { state: 'authorizing' }
  | { state: 'connected'; accountId: string; name: string }
  | { state: 'cancelled' }
  | { state: 'failed'; message: string }

export type SkinModel = 'classic' | 'slim'

export interface SkinAsset {
  id: string
  name: string
  model: SkinModel
  source: 'local' | 'microsoftProfile'
  width: number
  height: number
  createdAt: string
  updatedAt: string
}

/** Tipo de version instalada fuera del launcher (solo para nombrarla). */
export type ExternalKind = 'optifine' | 'forge' | 'neoforge' | 'fabric' | 'quilt' | 'liteloader' | 'other'

/** Version personalizada encontrada en una carpeta .minecraft (OptiFine, Forge puestos a mano...). */
export interface ExternalVersion {
  id: string
  baseVersion: string | null
  kind: ExternalKind
  releaseTime: string | null
  /** Ya se importo antes al launcher. */
  imported: boolean
}

export interface ExternalScan {
  dir: string
  /** `false` si la carpeta no existe o no tiene versiones. */
  found: boolean
  versions: ExternalVersion[]
}
