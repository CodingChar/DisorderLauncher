# 00 — MASTER SPECIFICATION
# DisorderLaucher

## 1. Misión

Construir un launcher de Minecraft Java ligero, modular, seguro y usable por personas con poca experiencia informática.

El producto debe poder:

- descubrir versiones;
- instalar Minecraft;
- administrar Java;
- crear instancias;
- ejecutar Vanilla;
- soportar Fabric, Forge, NeoForge y Quilt;
- gestionar mods/modpacks;
- reparar instalaciones;
- administrar perfiles Offline Gaming;
- añadir Microsoft posteriormente sin reescribir el núcleo;
- crecer hacia temas JSON y editor de skins.

---

## 2. Decisión irreversible de autenticación

### 2.1 Primera implementación: Offline Gaming

La primera experiencia funcional de identidad será Offline Gaming.

Debe permitir:

- crear perfil local;
- elegir nombre visible;
- generar y persistir identificador local estable;
- asignar skin local;
- seleccionar perfil predeterminado;
- iniciar instancias compatibles con identidad local;
- operar sin depender de OAuth.

### 2.2 Microsoft: posterior, pero anticipado desde Sprint 0

Microsoft no se implementará primero, pero la arquitectura debe quedar lista para:

```text
Microsoft OAuth
  -> Xbox identity
  -> XSTS
  -> Minecraft Services
  -> Minecraft profile / entitlement state
  -> GameIdentity
```

Por ello, desde el primer commit deben existir:

- `IdentityProvider`;
- `IdentityProviderKind`;
- `AccountRecord`;
- `ResolvedIdentity`;
- `AuthState`;
- `CredentialStore`;
- `AccountRepository`;
- provider registry;
- contratos de UI independientes del proveedor;
- eventos de autenticación;
- pruebas de contrato.

No se permite diseñar Offline Gaming con estructuras que luego obliguen a migrar todas las instancias cuando llegue Microsoft.

---

## 3. Stack

### Core
- Rust estable.
- Tokio.
- Reqwest.
- Serde.
- thiserror.
- anyhow solo en fronteras de aplicación.
- tracing.
- uuid.
- sha1/sha2 según metadatos.
- zip/tar.
- directories.
- keyring o integración nativa equivalente.

### Desktop
- Tauri 2.

### Frontend
- Svelte + TypeScript.
- CSS propio.
- tokens CSS centralizados.
- sin framework UI pesado.

### Persistencia
- JSON/TOML para configuración y manifiestos.
- almacenamiento seguro del SO para secretos.
- SQLite solo cuando exista una necesidad transaccional real.

---

## 4. Principios de diseño del código

1. El frontend nunca construye argumentos de launch.
2. El frontend nunca almacena access/refresh tokens.
3. `minecraft-launch` no conoce OAuth.
4. `minecraft-auth` no conoce Fabric/Forge.
5. Una `Instance` referencia `profile_id`.
6. `profile_id` se resuelve en tiempo de launch.
7. Un perfil puede cambiar de proveedor sin modificar archivos del juego.
8. Las credenciales no forman parte de exports de instancias.
9. Las skins locales no se confunden con skins oficiales.
10. El sistema de temas no ejecuta código.

---

## 5. Arquitectura de alto nivel

```text
Svelte UI
   │
Tauri IPC / events
   │
Application Services
   │
launcher-core
   ├── account-service
   ├── instance-service
   ├── install-service
   ├── launch-service
   ├── repair-service
   └── content-service
   │
Domain crates
   ├── identity-api
   ├── identity-local
   ├── identity-microsoft (stub inicial)
   ├── minecraft-meta
   ├── minecraft-install
   ├── minecraft-launch
   ├── java-runtime
   ├── downloader
   ├── instances
   ├── loaders
   ├── mods
   ├── modpacks
   ├── themes
   ├── skins
   └── diagnostics
```

---

## 6. Identidad normalizada

El launch engine consume una identidad resuelta, no un proveedor.

```rust
pub enum IdentityProviderKind {
    Local,
    Microsoft,
}

pub struct ResolvedIdentity {
    pub profile_id: String,
    pub provider: IdentityProviderKind,
    pub display_name: String,
    pub game_uuid: String,
    pub session: GameSessionMaterial,
    pub skin: Option<SkinReference>,
}
```

`GameSessionMaterial` encapsula únicamente el material necesario para el launch soportado.

Nunca exponer este objeto completo a la UI.

---

## 7. Estados de cuenta

No usar un simple booleano `loggedIn`.

```rust
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
```

La UI debe distinguir disponibilidad de identidad de autenticación online.

---

## 8. Instancias

Cada instancia debe quedar aislada.

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
```

No guardar tokens dentro de la instancia.

---

## 9. Minecraft y versiones

No mantener lista manual.

Resolver:

```text
version manifest
 -> version metadata
 -> client
 -> libraries
 -> asset index
 -> assets
 -> natives
 -> java requirement
 -> JVM/game arguments
```

Preparar un `InstallPlan` verificable antes de modificar la instalación.

---

## 10. Microsoft como riesgo externo

La integración Microsoft tiene dos capas de riesgo:

1. protocolo OAuth/Xbox/Minecraft Services;
2. habilitación/aceptación de la app registration para Minecraft Services.

La arquitectura debe permitir desarrollar y lanzar Offline Gaming aunque Microsoft todavía no esté habilitado para producción.

Debe existir un **Microsoft feasibility spike temprano** para validar:
- app registration;
- redirect URI;
- OAuth PKCE;
- scopes;
- llegada hasta Xbox/XSTS;
- respuesta de Minecraft Services.

Ese spike no convierte Microsoft en prioridad funcional; evita descubrir un bloqueo externo al final del proyecto.

---

## 11. UX

El usuario no técnico debe poder:

```text
Abrir launcher
 -> Crear perfil Offline Gaming
 -> Crear instancia
 -> Elegir versión
 -> Jugar
```

La configuración avanzada debe ser opcional.

---

## 12. MVP

MVP obligatorio:

- bootstrap completo;
- perfiles Offline Gaming;
- contrato Microsoft preparado;
- Vanilla;
- Java automático;
- downloader;
- assets/libraries/natives;
- instancias;
- launch;
- repair;
- logs;
- UI definitiva;
- Fabric;
- QA mínimo.

---

## 13. 1.0

- Forge;
- NeoForge;
- Quilt;
- mods;
- Modrinth;
- modpacks;
- Microsoft funcional cuando la app registration sea viable;
- secure credential storage;
- diagnósticos;
- updater;
- themes internos.

---

## 14. Futuro

- themes JSON de usuario;
- catálogo opcional de themes;
- skins local library;
- editor de skins;
- previews 3D;
- export/import.

---

## 15. Restricciones

Prohibido:

- acoplar instancias a tokens;
- usar secretos de cliente en desktop;
- almacenar refresh tokens en localStorage;
- mezclar perfil local con estado Microsoft;
- descargar desde mirrors no autorizados como fuente primaria;
- ejecutar CSS/JS arbitrario desde themes;
- silenciar errores críticos;
- hardcodear Java por versión si metadata permite resolverlo.
