# 02 — Arquitectura

## 1. Capas

```text
Presentation
  Svelte components / routes / stores

Application
  commands / use cases / orchestration

Domain
  identity / instances / minecraft / loaders / skins / themes

Infrastructure
  HTTP / filesystem / secure storage / process spawn / OS integration
```

## 2. Flujo de launch

```text
User presses Play
 -> LaunchInstance(instance_id)
 -> InstanceRepository.get()
 -> AccountRepository.get(instance.profile_id)
 -> IdentityRegistry.resolve(provider)
 -> provider.resolve_identity(account)
 -> VersionResolver.resolve()
 -> JavaManager.resolve()
 -> LaunchPlanner.build()
 -> LaunchValidator.validate()
 -> ProcessRunner.spawn()
 -> events -> UI
```

La UI nunca entrega manualmente username/token/uuid al motor.

## 3. Registry de providers

```rust
pub struct IdentityRegistry {
    providers: HashMap<IdentityProviderKind, Arc<dyn IdentityProvider>>,
}
```

Esto permite:
- Local hoy;
- Microsoft después;
- pruebas con fake provider.

## 4. Repositorios

```rust
trait AccountRepository
trait InstanceRepository
trait ThemeRepository
trait SkinRepository
```

Implementación inicial filesystem.

No mezclar I/O con modelos de dominio.

## 5. IPC

Comandos recomendados:

```text
accounts.list
accounts.create_local
accounts.set_default
accounts.microsoft.begin_login
accounts.microsoft.cancel_login
accounts.microsoft.logout

instances.list
instances.create
instances.update
instances.delete
instances.launch
instances.repair

minecraft.list_versions
minecraft.install

loaders.list_versions

mods.search
mods.install

themes.list
themes.apply

skins.list
skins.import
```

Los comandos Microsoft pueden existir detrás de feature flag antes de ser funcionales.

## 6. Eventos

```text
install://progress
install://phase
launch://stdout
launch://stderr
launch://state
auth://state
auth://browser-opened
auth://callback-received
repair://progress
```

## 7. Concurrencia

Downloader:
- Eco: 3;
- Normal: 6;
- Rendimiento: 10.

No usar concurrencia sin límites.

## 8. Atomicidad

Descargas:
1. escribir `.part`;
2. verificar;
3. fsync cuando proceda;
4. rename atómico.

Configuración:
- write temp;
- flush;
- rename.

## 9. Errores

Errores tipados por dominio.

`IdentityError` no debe convertirse prematuramente en string.

Ejemplos:
- LocalProfileInvalid
- MicrosoftNotConfigured
- MicrosoftAuthorizationCancelled
- MicrosoftStateMismatch
- MicrosoftTokenExchangeFailed
- MicrosoftXboxAuthFailed
- MicrosoftXstsFailed
- MinecraftServicesRejectedApp
- MinecraftOwnershipUnavailable
- SecureStoreUnavailable
- NetworkUnavailable

## 10. Telemetría

Por defecto:
- solo logs locales.
- no enviar información sin consentimiento.

Redactar secretos automáticamente.

---

## 11. Flujos técnicos detallados

### Offline launch

```text
Play
 -> Instance
 -> profile_id
 -> AccountRecord(Local)
 -> LocalIdentityProvider
 -> ResolvedIdentity
 -> VersionDescriptor
 -> Java
 -> LaunchPlan
 -> Process
```

### Microsoft launch futuro

```text
Play
 -> Instance
 -> profile_id
 -> AccountRecord(Microsoft)
 -> MicrosoftIdentityProvider
    -> cached session valid?
       yes -> ResolvedIdentity
       no  -> refresh token
              -> Microsoft token
              -> Xbox
              -> XSTS
              -> Minecraft Services
              -> profile/ownership
              -> ResolvedIdentity
 -> same VersionDescriptor
 -> same Java
 -> same LaunchPlan builder
 -> same ProcessRunner
```

La mitad inferior del flujo es idéntica. Esa es la razón principal para anticipar Microsoft en el diseño.

### Account lifecycle

```text
Create Local
 -> LocalReady

Connect Microsoft
 -> Authorizing
 -> Authenticated
 -> RefreshRequired
 -> Authenticated
 or Expired
```

### Failure boundary

Errores de identidad no deben corromper: instancia, assets, libraries, mods, saves.

### Offline availability

```text
identity ready?
 + installation complete?
 + Java available?
 = can launch without network
```

### Microsoft without network

No inventar una sesión nueva. Si existe identidad cacheada válida para una función offline admitida por la política del producto, representarla con estado explícito; si no, pedir reconexión.
