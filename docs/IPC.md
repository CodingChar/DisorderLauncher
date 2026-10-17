# Contrato IPC (Tauri)

Implementado en `apps/desktop/src-tauri/src/commands.rs`; la logica vive en `crates/launcher-core`.
Todos los comandos devuelven `Err(String)` con un mensaje listo para mostrar al usuario.
Los objetos se serializan en camelCase. Ningun comando devuelve ni recibe tokens.

## Comandos

| Comando | Argumentos | Resultado |
|---|---|---|
| `accounts_list` | — | `AccountRecord[]` |
| `accounts_default` | — | `string \| null` (id del perfil predeterminado) |
| `accounts_create_local` | `name` | `AccountRecord` (el primero queda como predeterminado) |
| `accounts_set_default` | `id` | `void` |
| `instances_list` | — | `Instance[]` |
| `instances_create` | `name`, `version`, `loader?`, `loaderVersion?` | `Instance` (perfil predeterminado). `loader`: `vanilla` (por defecto), `fabric` o `quilt`; sin `loaderVersion` se usa la ultima estable |
| `loaders_supported` | — | `LoaderKind[]` (siempre incluye `vanilla`) |
| `loaders_list_versions` | `loader`, `minecraftVersion` | `{ version, stable }[]` |
| `instances_update` | `id`, `changes` | `Instance` — `changes` (todos opcionales): `name`, `minecraftVersion`, `profileId`, `java` (`{mode:'auto'}` o `{mode:'path', path}`), `memory` (`{mode:'auto'}` o `{mode:'fixed', minMb, maxMb}`) |
| `instances_delete` | `id` | `void` (borra la instancia y su carpeta) |
| `instances_duplicate` | `id` | `Instance` (copia los archivos del juego) |
| `instances_launch` | `id` | `void` — inicia en segundo plano; el avance llega por eventos |
| `instances_stop` | `id` | `void` |
| `instances_repair` | `id` | `{ found, fixed, remaining }` — verifica hashes y redescarga lo danado |
| `mods_list` | `id` | `{ mods, missingDependencies, supported }` — cada mod: `fileName`, `enabled`, `name`, `version`, `description`, `size`, `source?` |
| `mods_set_enabled` | `id`, `file`, `enabled` | `void` (renombra a `.jar.disabled`) |
| `mods_remove` | `id`, `file` | `void` |
| `mods_add_local` | `id` | `ModEntry \| null` — abre el selector de archivos del sistema; `null` si se cancela |
| `mods_search` | `id`, `query`, `page?` | `ProjectHit[]` filtrados por version y loader de la instancia (Modrinth) |
| `mods_install` | `id`, `projectId` | `InstalledMod[]` (el mod y sus dependencias obligatorias) |
| `mods_check_updates` | `id` | `{ file, projectId, title, currentVersion, latestVersion }[]` |
| `mods_update` | `id`, `file` | `InstalledMod` (guarda la version anterior) |
| `mods_rollback` | `id`, `file` | `void` (restaura la version anterior) |
| `settings_get` | — | `{ settings, dataDir }` |
| `settings_update` | `settings` | `{ settings, dataDir }` — `settings`: `theme` (`system`/`light`/`dark`), `downloadProfile` (`eco`=3, `normal`=6, `performance`=10 descargas), `javaPath` (carpeta de Java global o `null`) |
| `minecraft_list_versions` | `includeAll?` | `VersionEntry[]` (solo `release` salvo `includeAll`) |

Nombre de perfil local: 3–16 caracteres, letras ASCII, numeros y `_`; sin duplicados (sin distinguir mayusculas).

## Eventos

| Evento | Payload |
|---|---|
| `install://progress` | `{ instanceId, phase, completed, total }` — `phase`: `client`, `libraries`, `assetIndex`, `assets`, `natives`, `verify` |
| `repair://progress` | igual que `install://progress` |
| `launch://state` | `{ instanceId, state }` — `state`: `preparing`, `installing`, `preparingJava`, `starting`, `running` (+`pid`), `exited` (+`code`), `failed` (+`message`) |
| `launch://stdout` / `launch://stderr` | `{ instanceId, line }` (secretos redactados; tambien se guarda en `instances/<id>/logs/latest.log`) |

## Notas de comportamiento

- Si la version ya esta instalada (descriptor + archivos presentes) no se usa la red: se puede jugar sin conexion.
- Si faltan archivos y no hay red, `launch://state` emite `failed` con "Necesitas conexion a internet...".
- Un fallo de identidad (p. ej. Microsoft no configurado) se reporta tal cual; nunca se degrada a perfil local.

## Pendiente conocido

- El progreso del instalador es por fase (conteo de archivos), no por bytes.
