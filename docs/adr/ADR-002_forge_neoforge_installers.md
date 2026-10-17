# ADR-002 — Forge y NeoForge mediante el instalador oficial

## Contexto

Forge y NeoForge no publican un perfil de version "listo" como Fabric/Quilt: distribuyen un **instalador** (`*-installer.jar`) que descarga librerias, ejecuta *processors* (parcheo del cliente con mappings) y escribe `versions/<id>/<id>.json`. Reimplementar los processors seria fragil y se desactualiza con cada version.

## Decision

Ejecutar el instalador oficial en modo `--installClient <data_dir>` con el Java gestionado, y consumir el perfil que genera igual que el de Fabric (`LoaderProfile` → `LaunchOverlay`).

Salvaguardas (spec 11):

- El instalador solo se descarga por HTTPS desde `maven.neoforged.net` / `maven.minecraftforge.net` (allowlist) y se verifica contra el `.sha1` publicado junto al artefacto.
- Se lanza con `Command` y argv (sin shell), directorio de trabajo temporal propio, entorno reducido, tiempo maximo y salida acotada (solo las ultimas lineas en el error).
- El instalador necesita un `launcher_profiles.json` minimo en el directorio destino; se crea vacio si no existe y no contiene datos del usuario.
- Un fallo no deja la instancia en un estado a medias: el perfil se considera instalado solo si existe su JSON y todas las librerias que declara.

## Consecuencias

- Requiere Java para *instalar* (no solo para jugar): el selector de Java debe ejecutarse antes que el loader.
- Los argumentos JVM modernos usan `${library_directory}` y `${classpath_separator}`; el constructor de lanzamiento los resuelve.
- No se puede probar de forma 100 % automatica sin red: la verificacion real es una prueba `#[ignore]` (como Fabric).
