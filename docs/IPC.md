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
| `instances_create` | `name`, `version` | `Instance` (Vanilla, perfil predeterminado) |
| `instances_update` | `id`, `changes` | `Instance` — `changes` (todos opcionales): `name`, `minecraftVersion`, `profileId`, `java` (`{mode:'auto'}` o `{mode:'path', path}`), `memory` (`{mode:'auto'}` o `{mode:'fixed', minMb, maxMb}`) |
| `instances_delete` | `id` | `void` (borra la instancia y su carpeta) |
| `instances_duplicate` | `id` | `Instance` (copia los archivos del juego) |
| `instances_launch` | `id` | `void` — inicia en segundo plano; el avance llega por eventos |
| `instances_stop` | `id` | `void` |
| `instances_repair` | `id` | `{ found, fixed, remaining }` — verifica hashes y redescarga lo danado |
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
