# DisorderLauncher — Plan Tecnico de Implementacion

> **Estado**: todas las tareas S0–S10 estan implementadas (salvo las bloqueadas por infraestructura). Ver [`PROGRESS.md`](PROGRESS.md) para el avance real, lo verificado y los pendientes; este documento se conserva como plan original.

Este documento es el plan maestro para construir DisorderLauncher. Esta disenado para que agentes de IA de menor consumo (Sonnet, Haiku) puedan ejecutar cada tarea sin leer los 14 documentos de especificacion completos.

**Regla**: antes de cada tarea, el agente debe leer `.claude/CLAUDE.md` y la seccion relevante de este plan. Solo consultar `docs/spec/` cuando necesite detalle adicional.

---

## Stack

| Capa | Tecnologia |
|------|-----------|
| Core | Rust estable + Tokio + Reqwest + Serde + thiserror + tracing + uuid + sha1/sha2 + zip |
| Desktop | Tauri 2 |
| Frontend | Svelte 5 + TypeScript strict + CSS custom properties |
| Persistencia | JSON/TOML (config), keyring/DPAPI/Keychain (secretos) |

---

## Estructura del workspace

```
DisorderLauncher/
├── Cargo.toml                  (workspace root)
├── rust-toolchain.toml         (stable)
├── apps/
│   └── desktop/                (Tauri 2 app)
│       ├── src-tauri/
│       │   ├── Cargo.toml
│       │   ├── src/
│       │   │   ├── main.rs
│       │   │   ├── commands/   (IPC handlers)
│       │   │   ├── state.rs    (AppState con servicios)
│       │   │   └── events.rs   (event emitters)
│       │   └── tauri.conf.json
│       ├── src/                (Svelte frontend)
│       │   ├── lib/
│       │   │   ├── components/
│       │   │   ├── stores/
│       │   │   ├── types/
│       │   │   └── api/        (bindings a Tauri commands)
│       │   ├── routes/
│       │   └── app.css         (tokens CSS)
│       ├── package.json
│       ├── svelte.config.js
│       ├── tsconfig.json
│       └── vite.config.ts
├── crates/
│   ├── identity-api/           (traits + tipos compartidos)
│   ├── identity-local/         (implementacion Offline Gaming)
│   ├── identity-microsoft/     (stub feature-gated)
│   ├── account-storage/        (persistencia de cuentas en filesystem)
│   ├── credential-store/       (wrapper de keyring/secure storage)
│   ├── minecraft-meta/         (version manifest, metadata parsing)
│   ├── minecraft-install/      (InstallPlan, downloader, verificacion)
│   ├── minecraft-launch/       (LaunchPlan, ProcessRunner)
│   ├── java-runtime/           (deteccion, descarga, seleccion de Java)
│   ├── download-manager/       (HTTP downloads con progreso, retry, hash verify)
│   ├── instance-manager/       (CRUD de instancias, game directories)
│   ├── launcher-core/          (orquestacion: une todos los crates)
│   └── diagnostics/            (health checks, repair)
├── docs/
├── tests/                      (integration tests)
├── theme_examples/
└── .github/workflows/
```

---

## Tareas por Sprint

Cada tarea esta numerada `S{sprint}.T{numero}`. El agente ejecutor debe:
1. Leer la tarea completa.
2. Implementar.
3. Ejecutar `cargo clippy --all-targets` y `cargo test --workspace`.
4. Verificar que compila sin errores.
5. Hacer commit con mensaje descriptivo.

---

### Sprint 0 — Fundaciones

#### S0.T1 — Scaffolding del workspace Rust

**Que hacer**: Crear el workspace Cargo con todos los crates vacios (solo `lib.rs` con un comentario o un `pub mod`). Cada crate debe tener su `Cargo.toml` con las dependencias minimas declaradas pero no necesariamente usadas todavia.

**Archivos a crear**:
- `Cargo.toml` (workspace members)
- `rust-toolchain.toml` (channel = "stable")
- `crates/identity-api/Cargo.toml` + `src/lib.rs`
- `crates/identity-local/Cargo.toml` + `src/lib.rs`
- `crates/identity-microsoft/Cargo.toml` + `src/lib.rs`
- `crates/account-storage/Cargo.toml` + `src/lib.rs`
- `crates/credential-store/Cargo.toml` + `src/lib.rs`
- `crates/minecraft-meta/Cargo.toml` + `src/lib.rs`
- `crates/minecraft-install/Cargo.toml` + `src/lib.rs`
- `crates/minecraft-launch/Cargo.toml` + `src/lib.rs`
- `crates/java-runtime/Cargo.toml` + `src/lib.rs`
- `crates/download-manager/Cargo.toml` + `src/lib.rs`
- `crates/instance-manager/Cargo.toml` + `src/lib.rs`
- `crates/launcher-core/Cargo.toml` + `src/lib.rs`
- `crates/diagnostics/Cargo.toml` + `src/lib.rs`

**Dependencias clave del workspace** (en `[workspace.dependencies]`):
```toml
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", features = ["json", "stream"] }
thiserror = "2"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4", "v3"] }
sha1 = "0.10"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
directories = "6"
keyring = "3"
zip = "2"
tempfile = "3"
```

**Criterio de exito**: `cargo check --workspace` pasa sin errores.

---

#### S0.T2 — Scaffolding Tauri 2 + Svelte

**Que hacer**: Crear la app Tauri 2 con frontend Svelte + TypeScript dentro de `apps/desktop/`.

**Pasos**:
1. Dentro de `apps/desktop/`, inicializar un proyecto Svelte con Vite.
2. Inicializar Tauri 2 (`src-tauri/`).
3. Configurar `tauri.conf.json` con identifier `com.disorderlauncher.app`.
4. Agregar `apps/desktop/src-tauri` como miembro del workspace.
5. El frontend debe tener TypeScript strict (`"strict": true` en tsconfig).
6. Crear `app.css` con CSS custom properties para tokens de color (ver `docs/spec/08_VISUAL_DESIGN_SYSTEM.md` para valores exactos).

**Tokens CSS minimos** (tema oscuro por defecto):
```css
:root {
  --bg: #0C0C0C;
  --surface: #141414;
  --surface-alt: #1D1D1D;
  --surface-raised: #252525;
  --text-primary: #F0F0EA;
  --text-secondary: #A0A09A;
  --border: #2D2D2D;
  --border-strong: #4D4D4D;
  --action: #F3F3ED;
  --action-text: #0C0C0C;
  --r-xl: 26px;
  --r-lg: 18px;
  --r-md: 12px;
  --r-sm: 8px;
}
:root[data-theme="light"] {
  --bg: #ECECE8;
  --surface: #F8F8F5;
  --surface-alt: #DFDFDB;
  --surface-raised: #CFCFCA;
  --text-primary: #0A0A0A;
  --text-secondary: #62625E;
  --border: #C8C8C2;
  --border-strong: #9E9E98;
  --action: #101010;
  --action-text: #F4F4EF;
}
```

**Tipografia**: Space Grotesk 600/700 (display), IBM Plex Sans 400/500/600 (body).

**Criterio de exito**: `cargo tauri dev` abre una ventana con el frontend Svelte mostrando "DisorderLauncher" en texto.

---

#### S0.T3 — Contratos de identidad (identity-api)

**Que hacer**: Definir los tipos y traits centrales de identidad en `crates/identity-api/`.

**Codigo exacto a implementar**:

```rust
// crates/identity-api/src/lib.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdentityProviderKind {
    Local,
    Microsoft,
}

impl fmt::Display for IdentityProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Microsoft => write!(f, "microsoft"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedIdentity {
    pub profile_id: String,
    pub provider: IdentityProviderKind,
    pub display_name: String,
    pub game_uuid: String,
    pub session: GameSessionMaterial,
    pub skin: Option<SkinReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSessionMaterial {
    pub access_token: String,
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinReference {
    pub id: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthState {
    LocalReady,
    MicrosoftDisconnected,
    MicrosoftAuthorizing,
    MicrosoftAuthenticated,
    MicrosoftRefreshRequired,
    MicrosoftOfflineCachedIdentity,
    MicrosoftExpired,
    Error(AuthErrorKind),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthErrorKind {
    ProviderNotConfigured,
    NetworkUnavailable,
    ProfileInvalid(String),
    TokenExchangeFailed,
    SecureStoreUnavailable,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: String,
    pub provider: IdentityProviderKind,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub provider_data: ProviderAccountData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ProviderAccountData {
    #[serde(rename = "local")]
    Local(LocalAccountData),
    #[serde(rename = "microsoft")]
    Microsoft(MicrosoftAccountMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAccountData {
    pub display_name: String,
    pub local_uuid: String,
    pub skin_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccountMetadata {
    pub minecraft_uuid: Option<String>,
    pub minecraft_name: Option<String>,
    pub secure_refresh_token_key: Option<String>,
    pub last_successful_auth: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("local profile invalid: {0}")]
    LocalProfileInvalid(String),
    #[error("microsoft not configured")]
    MicrosoftNotConfigured,
    #[error("microsoft authorization cancelled")]
    MicrosoftAuthorizationCancelled,
    #[error("microsoft token exchange failed: {0}")]
    MicrosoftTokenExchangeFailed(String),
    #[error("secure store unavailable: {0}")]
    SecureStoreUnavailable(String),
    #[error("network unavailable")]
    NetworkUnavailable,
    #[error("{0}")]
    Other(String),
}

#[async_trait::async_trait]
pub trait IdentityProvider: Send + Sync {
    fn kind(&self) -> IdentityProviderKind;
    async fn resolve_identity(&self, account: &AccountRecord) -> Result<ResolvedIdentity, IdentityError>;
    async fn refresh(&self, account: &AccountRecord) -> Result<AuthState, IdentityError>;
    async fn logout(&self, account: &AccountRecord) -> Result<(), IdentityError>;
}
```

**Criterio de exito**: `cargo test -p identity-api` pasa. Agregar al menos un test de serializacion/deserializacion de `AccountRecord`.

---

#### S0.T4 — LocalIdentityProvider (identity-local)

**Que hacer**: Implementar el provider de Offline Gaming.

**Codigo clave**:
- Funcion para generar UUID offline compatible con Java: `UUID.nameUUIDFromBytes("OfflinePlayer:" + username)` — esto es un UUID v3 con namespace MD5.
- `LocalIdentityProvider` que implementa `IdentityProvider`.
- `resolve_identity` produce un `ResolvedIdentity` con `access_token: "0"` y `token_type: "legacy"` (valores que el cliente Minecraft acepta para offline).

**Tests obligatorios**:
- UUID estable: el mismo nombre siempre produce el mismo UUID.
- Vectores conocidos: "Notch" -> UUID conocido (buscar en spec o generar).
- `resolve_identity` devuelve `ResolvedIdentity` correcto.
- `refresh` devuelve `AuthState::LocalReady`.
- `logout` devuelve `Ok(())`.

**Referencia**: `docs/spec/03_IDENTITY_AUTH_OFFLINE_FIRST.md`

---

#### S0.T5 — FakeMicrosoftProvider (identity-microsoft stub)

**Que hacer**: Crear un stub feature-gated que implementa `IdentityProvider` devolviendo `IdentityError::MicrosoftNotConfigured` en todas las operaciones.

**Cargo.toml**:
```toml
[features]
default = []
microsoft-auth = []
```

Cuando `microsoft-auth` no esta habilitado, el provider siempre devuelve `MicrosoftNotConfigured`.

**Tests**: contract test que verifica que el stub compila y devuelve el error esperado.

---

#### S0.T6 — IdentityRegistry + AccountRepository

**Que hacer**: En `launcher-core`, crear:

```rust
pub struct IdentityRegistry {
    providers: HashMap<IdentityProviderKind, Arc<dyn IdentityProvider>>,
}

impl IdentityRegistry {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, provider: Arc<dyn IdentityProvider>) { ... }
    pub fn get(&self, kind: IdentityProviderKind) -> Option<Arc<dyn IdentityProvider>> { ... }
}
```

En `account-storage`:
```rust
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<AccountRecord>>;
    async fn get(&self, id: &str) -> Result<Option<AccountRecord>>;
    async fn save(&self, record: &AccountRecord) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn get_default(&self) -> Result<Option<String>>;
    async fn set_default(&self, id: &str) -> Result<()>;
}
```

Implementacion `FileAccountRepository` que lee/escribe JSON en un directorio.

**Tests**: CRUD completo con directorio temporal.

---

#### S0.T7 — CredentialStore trait

**Que hacer**: En `credential-store/`:

```rust
#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn put_secret(&self, key: &str, secret: &str) -> Result<(), CredentialError>;
    async fn get_secret(&self, key: &str) -> Result<Option<String>, CredentialError>;
    async fn delete_secret(&self, key: &str) -> Result<(), CredentialError>;
}
```

Implementacion `KeyringCredentialStore` usando el crate `keyring`.
Implementacion `InMemoryCredentialStore` para tests.

No se usa todavia — se prepara para Sprint 6 (Microsoft).

---

### Sprint 1 — Offline Gaming + Vanilla end-to-end

#### S1.T1 — minecraft-meta: Version Manifest

**Que hacer**: Parsear el manifiesto de versiones de Mojang.

**URL**: `https://piston-meta.mojang.com/mc/game/version_manifest_v2.json`

**Tipos**:
```rust
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}
pub struct VersionEntry {
    pub id: String,
    pub r#type: VersionType, // release, snapshot, old_beta, old_alpha
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
}
```

Funciones:
- `fetch_manifest() -> Result<VersionManifest>` (HTTP)
- `load_cached_manifest(path) -> Result<VersionManifest>` (filesystem)
- `parse_version_json(url) -> Result<VersionDescriptor>` (el JSON completo de una version)

El `VersionDescriptor` contiene: client jar, libraries, asset index, assets, natives, Java requirement, JVM args, game args.

**Referencia**: `docs/spec/05_MINECRAFT_ENGINE.md`

---

#### S1.T2 — download-manager

**Que hacer**: Download manager con progreso, retry, verificacion de hash.

**API**:
```rust
pub struct DownloadTask {
    pub url: Url,
    pub dest: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
}

pub struct DownloadManager { ... }

impl DownloadManager {
    pub async fn download_all(
        &self,
        tasks: Vec<DownloadTask>,
        concurrency: usize,
        progress: impl Fn(DownloadProgress) + Send + Sync,
    ) -> Result<DownloadReport>;
}
```

Reglas:
- Escribir a `.part`, verificar hash, rename atomico.
- Concurrencia configurable: 3 (eco), 6 (normal), 10 (rendimiento).
- Retry hasta 3 veces con backoff.
- HTTPS obligatorio.

---

#### S1.T3 — minecraft-install: InstallPlan + instalacion

**Que hacer**: Construir un `InstallPlan` a partir de un `VersionDescriptor` y ejecutar la instalacion.

```rust
pub struct InstallPlan {
    pub version: String,
    pub client: Artifact,
    pub libraries: Vec<Artifact>,
    pub asset_index: Artifact,
    pub assets: Vec<Artifact>,
    pub natives: Vec<Artifact>,
    pub java_requirement: JavaRequirement,
}

pub struct Artifact {
    pub url: String,
    pub path: PathBuf,
    pub size: Option<u64>,
    pub sha1: Option<String>,
}
```

Fases: metadata -> validate -> client -> libraries -> asset index -> assets -> natives -> Java -> persist -> verify.

Emitir eventos de progreso por fase.

---

#### S1.T4 — java-runtime: deteccion y descarga

**Que hacer**: Detectar Java instalado, seleccionar version compatible, descargar runtime administrado si no hay uno compatible.

```rust
pub struct JavaManager { ... }

impl JavaManager {
    pub async fn detect_installed(&self) -> Vec<JavaInstallation>;
    pub async fn select_for_version(&self, requirement: &JavaRequirement) -> Result<JavaInstallation>;
    pub async fn download_runtime(&self, requirement: &JavaRequirement) -> Result<JavaInstallation>;
}
```

Fuente de runtimes administrados: Adoptium API (https://api.adoptium.net).

---

#### S1.T5 — minecraft-launch: LaunchPlan + ProcessRunner

**Que hacer**: Construir argumentos JVM y game, y ejecutar Minecraft.

```rust
pub struct LaunchPlan {
    pub java_path: PathBuf,
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
}

pub struct LaunchPlanBuilder { ... }

impl LaunchPlanBuilder {
    pub fn build(
        version: &VersionDescriptor,
        identity: &ResolvedIdentity,
        java: &JavaInstallation,
        instance_dir: &Path,
    ) -> Result<LaunchPlan>;
}
```

`ProcessRunner`:
- Spawn directo (no shell).
- Capturar stdout/stderr como streams.
- Emitir eventos.
- Exit code.

**Sustitucion de placeholders**: `${auth_player_name}`, `${auth_uuid}`, `${auth_access_token}`, `${auth_session}`, `${game_directory}`, `${assets_root}`, `${assets_index_name}`, `${user_type}`, `${version_name}`, `${version_type}`.

---

#### S1.T6 — instance-manager

**Que hacer**: CRUD de instancias.

```rust
pub struct Instance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: Option<String>,
    pub profile_id: String,
    pub java: JavaSelection,
    pub memory: MemoryPolicy,
    pub game_dir: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum LoaderKind { Vanilla, Fabric, Forge, NeoForge, Quilt }
pub enum JavaSelection { Auto, Path(PathBuf) }
pub enum MemoryPolicy { Auto, Fixed { min_mb: u32, max_mb: u32 } }
```

Persistencia: `instances/<id>/instance.json` + `game/` subdirectory.

---

#### S1.T7 — Tauri IPC: comandos basicos

**Que hacer**: Exponer comandos Tauri para el frontend.

Comandos iniciales:
```rust
#[tauri::command]
async fn accounts_list(state: State<'_, AppState>) -> Result<Vec<AccountRecord>, String>;

#[tauri::command]
async fn accounts_create_local(state: State<'_, AppState>, name: String) -> Result<AccountRecord, String>;

#[tauri::command]
async fn accounts_set_default(state: State<'_, AppState>, id: String) -> Result<(), String>;

#[tauri::command]
async fn instances_list(state: State<'_, AppState>) -> Result<Vec<Instance>, String>;

#[tauri::command]
async fn instances_create(state: State<'_, AppState>, name: String, version: String) -> Result<Instance, String>;

#[tauri::command]
async fn instances_launch(state: State<'_, AppState>, id: String) -> Result<(), String>;

#[tauri::command]
async fn minecraft_list_versions(state: State<'_, AppState>) -> Result<Vec<VersionEntry>, String>;
```

`AppState` contiene: `AccountRepository`, `InstanceRepository`, `IdentityRegistry`, `JavaManager`, etc.

Eventos Tauri: `install://progress`, `launch://stdout`, `launch://stderr`, `launch://state`.

---

#### S1.T8 — Frontend: UI de primer vertical slice

**Que hacer**: Construir la UI minima que permite el flujo completo.

**Pantallas**:

1. **Welcome** (primera ejecucion): campo de nombre + boton "Crear perfil".
2. **Home**: instancia activa con boton JUGAR, version, estado. Lista de instancias.
3. **Create Instance**: selector de version + nombre + boton crear.
4. **Account Picker**: lista de perfiles, boton usar, indicador de tipo (Offline Gaming).

**Componentes Svelte** (`apps/desktop/src/lib/components/`):
- `Button.svelte` (primario, secundario)
- `Input.svelte`
- `Select.svelte`
- `Card.svelte`
- `ProfileBadge.svelte`
- `InstanceCard.svelte`
- `ProgressBar.svelte`

**Stores** (`apps/desktop/src/lib/stores/`):
- `accounts.ts` (lista de cuentas, cuenta activa)
- `instances.ts` (lista de instancias, instancia activa)
- `versions.ts` (catalogo de versiones)

**Bindings** (`apps/desktop/src/lib/api/`):
- `invoke('accounts_list')`, `invoke('accounts_create_local', { name })`, etc.
- Listeners para eventos de progreso.

**Referencia visual**: `docs/reference/disorderlauncher_reference.html`

---

#### S1.T9 — Repair basico

**Que hacer**: Verificar integridad de archivos de una instancia y redescargar faltantes/corruptos.

En `diagnostics/`:
```rust
pub async fn verify_instance(instance: &Instance, version: &VersionDescriptor) -> VerifyReport;
pub async fn repair_instance(instance: &Instance, report: &VerifyReport) -> Result<RepairReport>;
```

---

### Sprint 2 — Instancias + UX de producto

#### S2.T1 — CRUD completo de instancias en UI
- Editar nombre, version, perfil asociado, Java override, RAM.
- Duplicar instancia.
- Eliminar instancia (con confirmacion).

#### S2.T2 — Settings page
- Tema claro/oscuro.
- Directorio de datos.
- Concurrencia de descargas.
- Java global.

#### S2.T3 — Error dialogs y estados vacios
- Dialogo de error generico con mensaje + accion.
- Empty states: sin instancias, sin perfiles, sin versiones.

#### S2.T4 — Accesibilidad
- Focus visible en todos los interactivos.
- Targets >= 40px.
- Navegacion por teclado.
- `prefers-reduced-motion`.

---

### Sprint 3 — Fabric + Mods + Modrinth

#### S3.T1 — LoaderProvider trait + Fabric

**Que hacer**: Crear el trait de loaders e implementar Fabric.

```rust
#[async_trait]
pub trait LoaderProvider: Send + Sync {
    fn kind(&self) -> LoaderKind;
    async fn versions(&self, minecraft_version: &str) -> Result<Vec<LoaderVersion>>;
    async fn install(&self, ctx: LoaderInstallContext) -> Result<InstalledLoader>;
    async fn launch_overlay(&self, ctx: LoaderLaunchContext) -> Result<LaunchOverlay>;
}
```

Fabric API: `https://meta.fabricmc.net/v2/versions/loader/{mc_version}`

`LaunchOverlay` modifica el classpath y main class del `LaunchPlan`.

#### S3.T2 — Mod manager
- Listar mods de una instancia (scan `mods/`).
- Enable/disable (renombrar `.jar` <-> `.jar.disabled`).
- Eliminar mod.

#### S3.T3 — Modrinth integration
- Buscar mods: `https://api.modrinth.com/v2/search`
- Filtrar por game_version + loader.
- Descargar mod + resolver dependencias.

#### S3.T4 — UI de mods
- Pantalla de mods por instancia.
- Buscador Modrinth integrado.
- Boton instalar/actualizar/eliminar.

---

### Sprint 4 — NeoForge + Forge + Quilt

#### S4.T1 — NeoForge LoaderProvider
#### S4.T2 — Forge LoaderProvider (con processor pipeline)
#### S4.T3 — Quilt LoaderProvider
#### S4.T4 — UI de seleccion de loader en creacion de instancia

---

### Sprint 5 — Modpacks

#### S5.T1 — Formato interno de modpack
#### S5.T2 — Importador .mrpack (Modrinth)
#### S5.T3 — Export de instancia como modpack
#### S5.T4 — UI de modpacks

---

### Sprint 6 — Microsoft (cuando app registration sea viable)

#### S6.T1 — OAuth PKCE flow en identity-microsoft
#### S6.T2 — Xbox + XSTS + Minecraft Services chain
#### S6.T3 — Secure token storage con credential-store
#### S6.T4 — Refresh flow + error handling
#### S6.T5 — Account picker UI con Microsoft
#### S6.T6 — Ownership verification

**Referencia detallada**: `docs/spec/04_MICROSOFT_AUTH_FUTURE.md`

---

### Sprint 7 — Themes v1

#### S7.T1 — Theme engine: validar JSON, aplicar CSS variables
#### S7.T2 — Theme selector UI
#### S7.T3 — Import/preview de themes custom

**Schema**: `theme_examples/theme-schema-v1.json`

---

### Sprint 8 — Skins library

#### S8.T1 — Skin import, validacion, storage
#### S8.T2 — Skin preview (2D)
#### S8.T3 — Asociar skin a perfil local

---

### Sprint 9 — Skin editor

#### S9.T1 — Canvas editor (pencil, eraser, fill, picker)
#### S9.T2 — Undo/redo + layers
#### S9.T3 — 3D preview
#### S9.T4 — Export/save

---

### Sprint 10 — Hardening y release

#### S10.T1 — Packaging (instaladores Windows/macOS/Linux)
#### S10.T2 — Auto-updater
#### S10.T3 — Schema migrations
#### S10.T4 — Security audit
#### S10.T5 — Performance profiling
#### S10.T6 — QA matrix completa

---

## Reglas para agentes ejecutores

1. **Nunca** almacenar tokens en el frontend o localStorage.
2. **Nunca** usar `sh -c` o `cmd /c` para launch — siempre spawn directo con argv.
3. **Nunca** ejecutar codigo desde themes (solo datos JSON).
4. **Siempre** verificar hashes de descargas.
5. **Siempre** escribir a `.part` + rename atomico para descargas y config.
6. **Siempre** normalizar paths de ZIP y prohibir `..`.
7. Errores tipados por dominio — no convertir en `String` prematuramente.
8. El frontend llama a comandos Tauri; nunca construye argumentos de launch.
9. Una `Instance` referencia `profile_id`, nunca contiene tokens ni username duplicado.
10. `cargo clippy --all-targets` y `cargo test --workspace` deben pasar despues de cada tarea.
