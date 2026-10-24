# Integridad de instancias y robustez con modpacks

Codigo: `crates/instance-manager/src/repository.rs`, `crates/launcher-core/src/{integrity,crash_hints,recommendation,memory,modpack_ops}.rs`, `crates/account-storage`.

## Que pasa si algo se borra o se corta desde fuera

La lista de instancias sale **siempre del disco** (`instances/<id>/instance.json`), no de una copia en memoria. La interfaz la relee al arrancar y cada vez que la ventana recupera el foco, asi que una instancia borrada desde el explorador desaparece sola de la lista; lanzarla antes de eso da «No se encontro la instancia».

| Situacion | Resultado |
|---|---|
| Se borra la carpeta de una instancia | Desaparece de la lista. Nada que limpiar. |
| Se borran `game/`, sus subcarpetas o `logs/` | `ensure_layout` las recrea antes de lanzar y en la revision de arranque. |
| Borrado incompleto (archivo bloqueado, juego abierto) | `delete` quita primero `instance.json`, asi la instancia ya no aparece; la carpeta sobrante se limpia como huerfana en el siguiente arranque (`scan` + `remove_orphan`, que nunca toca una instancia valida). |
| `instance.json` danado | Se restaura de `instance.json.bak` (copia del ultimo guardado valido). Un formato mas nuevo o un id distinto **no** se «arreglan» con la copia: se informan. |
| El launcher se cierra durante una importacion de modpack | La instancia lleva `.disorder/import.pending`; al arrancar se deshace (rollback) y no se puede lanzar mientras tanto. |
| El perfil de la instancia desaparece | Se avisa («A «X» le falta su perfil…») y el lanzamiento da un mensaje claro. **Nunca** se cambia a otro perfil en silencio. |
| El Java elegido a mano ya no existe | Se avisa para volver a elegir. |
| Una ficha ilegible sin copia | Se deja intacta y se avisa. |

`Launcher::instances_integrity` hace la revision (solo de instancias que no estan en uso) y `startup_notices` convierte el resultado en avisos para la persona; las reparaciones silenciosas solo se registran en el log.

## Perfiles: causa del «no se encuentra el perfil»

`FileAccountRepository` escribia siempre a `<id>.json.part` y lo renombraba. Dos guardados a la vez del mismo perfil (cerrar una instancia y abrir otra) compartian el temporal, y el segundo podia dejar un JSON a medias que la lectura siguiente apartaba como `.corrupt`, con lo que el perfil «desaparecia». Ademas, el lanzamiento guardaba el perfil entero al final de la preparacion y pisaba cambios hechos mientras tanto (p. ej. la skin).

- Escrituras serializadas con un candado y temporal propio (`<id>.json.<pid>-<n>.part`); `_default.txt` igual.
- Antes de apartar un archivo ilegible se vuelve a leer una vez.
- `touch_used` relee el perfil actual y solo cambia `lastUsedAt`.
- Pruebas: `concurrent_saves_never_leave_a_torn_or_quarantined_file`, `touch_used_keeps_changes_made_after_the_profile_was_read`.

## Modpacks

- **Java**: las instancias con loader exigen el Java exacto del juego (`ensure_exact_java`); un Java mas nuevo rompe el Mixin/ASM de los loaders antiguos (Fabric Loader 0.14.x con Java 21: `Unsupported class file major version 65`).
- **Antes de importar**: se comprueba que la version de Minecraft exista y que haya espacio en disco (peso de los archivos + 25 % + 1 GB).
- **Memoria automatica proporcional a los mods**: 2 GB + 20 MB por mod, hasta el 60 % de la RAM del equipo y 8 GB (`auto_memory_for_mods`). Con menos de 20 mods no cambia.
- **Por que se cierra el juego** (`crash_hints`): si el juego termina con error y se reconoce la firma, el launcher muestra la causa y la accion (Java que no encaja, memoria insuficiente, mods incompatibles); gana la causa mas especifica (un Java demasiado nuevo acaba como «Mixin transformation failed» y se explica como Java).
- **Recomendado para mi equipo** (Opciones avanzadas): `instances_recommendation` sugiere nivel de rendimiento segun hilos, RAM y mods, con los motivos en texto simple; no cambia nada hasta que se guarda.
