# 07 — Loaders, mods y modpacks

## LoaderProvider

```rust
#[async_trait]
pub trait LoaderProvider {
    fn kind(&self) -> LoaderKind;
    async fn versions(&self, mc: &str) -> Result<Vec<LoaderVersion>>;
    async fn install(&self, ctx: LoaderInstallContext) -> Result<InstalledLoader>;
    async fn launch_overlay(&self, ctx: LoaderLaunchContext) -> Result<LaunchOverlay>;
}
```

El loader nunca recibe tokens de cuenta.

## Orden

1. Vanilla.
2. Fabric.
3. NeoForge.
4. Forge.
5. Quilt.

El orden puede ajustarse por dificultad real, pero Vanilla y Fabric primero.

## Mods

`ModManager` conoce:
- instance;
- Minecraft version;
- loader;
- filesystem.

No conoce identidad.

## Modrinth

Filtrar automáticamente por:
- game version;
- loader.

Resolver dependencias antes de escribir.

## Modpacks

Formato interno:
- declarativo;
- reproducible;
- hashes;
- overrides controlados.

Importadores:
- `.mrpack`;
- otros más tarde.

## Seguridad ZIP

Obligatorio:
- normalizar paths;
- prohibir `..`;
- prohibir salida del destino;
- límites de tamaño;
- límites de cantidad;
- evitar zip bombs cuando sea razonable.

## UX

Usuario no técnico:
- “Compatible”
- “No compatible con tu versión”
- “Necesita Fabric API”
- “Actualizar”

Evitar mostrar coordenadas Maven salvo modo avanzado.
