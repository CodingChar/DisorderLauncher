# 01 — Construcción desde cero

## 1. Repositorio

Nombre:
`DisorderLauncher`

Estructura inicial:

```text
DisorderLauncher/
├── Cargo.toml
├── rust-toolchain.toml
├── apps/
│   └── desktop/
├── crates/
├── docs/
├── tests/
├── scripts/
├── .github/
└── README.md
```

## 2. Workspace Rust

Crear workspace desde el principio.

Crates iniciales:

```text
launcher-core
identity-api
identity-local
identity-microsoft
account-storage
credential-store
minecraft-meta
minecraft-install
minecraft-launch
java-runtime
download-manager
instance-manager
diagnostics
```

`identity-microsoft` puede iniciar como feature-gated stub compilable, pero debe implementar el mismo trait.

## 3. Tauri + Svelte

Crear app Tauri 2 con Svelte/TypeScript.

Reglas:
- frontend sin acceso directo al filesystem;
- comandos Tauri tipados;
- eventos para progreso;
- validación de payloads en Rust;
- CSP restrictiva.

## 4. Quality gates desde el primer commit

Rust:
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features`
- `cargo test --workspace`

Frontend:
- TypeScript strict.
- lint.
- unit tests.
- build production.

## 5. Configuración

```text
LauncherData/
├── config/
│   ├── app.json
│   └── ui.json
├── profiles/
├── credentials/
├── instances/
├── cache/
├── metadata/
├── assets/
├── libraries/
├── runtimes/
├── themes/
├── skins/
└── logs/
```

`credentials/` no debe contener tokens en claro. Puede contener referencias/metadata no secreta del secure store.

## 6. Bootstrap de identidad

Antes de instalar Minecraft, implementar:

```rust
#[async_trait]
pub trait IdentityProvider {
    fn kind(&self) -> IdentityProviderKind;
    async fn resolve_identity(&self, account: &AccountRecord)
        -> Result<ResolvedIdentity, IdentityError>;
    async fn refresh(&self, account: &AccountRecord)
        -> Result<AuthState, IdentityError>;
    async fn logout(&self, account: &AccountRecord)
        -> Result<(), IdentityError>;
}
```

Local:
- implementado inmediatamente.

Microsoft:
- stub que devuelve `NotConfigured` hasta sprint correspondiente.

## 7. Primer vertical slice

El primer vertical slice debe ser:

```text
UI
 -> crear perfil local
 -> persistir cuenta
 -> crear instancia Vanilla
 -> resolver metadata
 -> instalar
 -> resolver identidad local
 -> LaunchPlan
 -> ejecutar
 -> mostrar logs
```

No construir pantallas aisladas antes de demostrar ese recorrido.

## 8. CI

Pipeline:
1. lint Rust;
2. tests Rust;
3. frontend lint/test;
4. frontend build;
5. Tauri check/build cuando plataforma lo permita;
6. artifact de desarrollo.

## 9. Feature flags

Sugeridos:

```toml
[features]
default = ["offline-auth"]
offline-auth = []
microsoft-auth = []
themes-user = []
skin-editor = []
```

El código no debe bifurcar arquitectura por feature; solo habilitar implementaciones.
