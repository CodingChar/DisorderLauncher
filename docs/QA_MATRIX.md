# Matriz de QA

Estado a 2026-10-17. Leyenda: ✅ verificado automaticamente · 🖐 verificado a mano en esta sesion · ⚠️ implementado, no verificado en ese entorno · ❌ no implementado.

## Plataformas

| Plataforma | Compila + tests | Flujo real (instalar y jugar) | UI en navegador |
|---|---|---|---|
| Windows 10 (x64) | ✅ | ✅ Vanilla 1.0 (Java 8), Fabric 1.14.4, Forge 1.14.4, NeoForge 1.21.1 (Java 21) | 🖐 Chromium (con backend simulado) |
| Linux | ⚠️ (CI configurado, sin ejecutar) | ⚠️ | ⚠️ |
| macOS | ⚠️ (sin CI) | ⚠️ | ⚠️ |
| Ventana nativa de Tauri (Windows) | ✅ build release + MSI | 🖐 arranque del `.exe` verificado via CDP: pantalla de bienvenida bajo la CSP de produccion, IPC real (`accounts_list`, `settings_get`, `themes_list`, `loaders_supported`, `microsoft_status`) y 0 errores de consola | 🖐 |

## Funciones

| Area | Automatizado | Real (red/juego) | Notas |
|---|---|---|---|
| Perfiles Offline Gaming | ✅ | — | UUID compatible con `OfflinePlayer:` |
| Perfiles Microsoft | ✅ contra servidor falso (PKCE, refresh, errores) | ❌ sin app registrada | Deshabilitado sin client id |
| Vanilla (install/launch/repair) | ✅ | ✅ 1.0 | Allowlist Mojang verificada con descarga real |
| Java gestionado (Adoptium) | ✅ checksum, seleccion | ✅ Java 8 | |
| Fabric / Quilt | ✅ | ✅ Fabric 1.14.4 y Quilt 1.14.4 arrancan en el juego; Fabric + Fabric API instalada desde Modrinth: el juego carga el mod | |
| NeoForge / Forge | ✅ versiones, URLs, checksum, ids | ✅ Forge 1.14.4 (Java 8) y NeoForge 1.21.1 (Java 21): el juego carga el loader | Via instalador oficial (ADR-002); mods administrados tambien en Forge/NeoForge (lectura de `mods.toml`, solo jars sinteticos) |
| Mods + Modrinth | ✅ resolver, locales, updates, lectura de Forge/NeoForge | ✅ (Fabric) busqueda, instalacion, actualizar, volver atras; ✅ Forge 1.20.1 y NeoForge 1.21.1: busqueda, instalacion de JEI desde Modrinth y lectura de su `mods.toml` | |
| Modpacks `.mrpack` | ✅ import/export/rollback | ✅ export+import con referencias Modrinth | |
| Temas | ✅ validacion y almacen (incluye el tema azul oscuro) | 🖐 aplicacion en UI (negro y azul) | |
| Skins + editor | ✅ backend, 18 tests de logica del editor | 🖐 editor, 3D, biblioteca; 🖐 teclado del lienzo en la ventana nativa (flechas, Mayús salta 8, limites, Intro pinta; teclas reales por CDP) | Skin local se aplica en el juego con un resource pack (🖐 1.12.2: el juego lo carga); el modelo de brazos lo elige Minecraft |
| Interfaz reformada (maqueta) | ✅ `svelte-check`, build | 🖐 todas las vistas en navegador (820x560 y 1100x720, negro y azul); 🖐 ventana nativa (`tauri dev`, 2026-10-17): Jugar en «Mi mundo» (Vanilla), la consola mostró la salida real del juego (28 líneas) y «Cerrar juego» lo detuvo | Ver `docs/UI_MIGRATION.md` |
| Versiones por tipo de instancia | ✅ `version_catalog_without_network_falls_back_to_installed_versions` | ✅ 919 versiones de Mojang con los 4 tipos; Fabric 505, Quilt 441, Forge 76, NeoForge 17 | Sin red: cache o instalado |
| Optimizacion (JVM + opciones + mods) | ✅ 4 pruebas | ✅ `performance_mods_install_by_slug`; 🖐 1.12.2 con «Equipo modesto» (SerialGC y `options.txt`) | |
| Ventana unica | — | 🖐 segunda ejecucion no crea otro proceso | |
| Descargas con redireccion | ✅ revalida https y host | — | |
| Recuperacion de datos | ✅ migracion, cuarentena | — | |

## Pruebas

- Rust: **257 pruebas pasan**, 0 fallan, 11 `#[ignore]` (requieren internet/ventana, se ejecutan a mano).
- Frontend: 18 pruebas (Vitest) de la logica del editor; `svelte-check` sin errores.
- Pruebas reales (manuales): `real_vanilla_install_launch_and_stop`, `real_fabric_install_launch_and_stop`, `real_quilt_install_launch_and_stop`, `real_fabric_with_modrinth_mod_loads_the_mod`, `real_forge_install_launch_and_stop`, `real_neoforge_install_launch_and_stop`, `real_fabric_install`, `real_quilt_install`, `real_modpack_export_and_import_use_modrinth_references`, `real_manifest_…`, y las de Modrinth (incluye `forge_mod_is_found_installed_and_read` y `neoforge_mod_is_found_installed_and_read`) en `crates/mods/tests/real_modrinth.rs`.

Recorrido completo por la interfaz del ejecutable release (2026-10-18, teclas y clics reales por CDP): crear instancia Fabric 1.20.1 con icono y rendimiento «Equilibrado» → instalar y jugar (la consola recibio 50 lineas del juego y la JVM llevo las banderas de G1) → cerrar el juego → eliminar la instancia (carpeta borrada).

Modo sin conexion (release con un proxy muerto para todo el trafico externo, 2026-10-18): el catalogo de versiones responde con el ultimo manifiesto guardado o con lo instalado (Forge sin lista guardada: «installed»), se crea una instancia 1.12.2 ya instalada y se lanza (proceso de Java a los 3 s) sin ninguna descarga.

Ultima corrida completa de las 19 pruebas reales (`cargo test --workspace -- --ignored --test-threads=1`, 2026-10-18): todas ok (Vanilla, Fabric, Quilt, Forge y NeoForge instalan y arrancan; Modrinth; modpacks; catalogo de versiones; rendimiento).

## Consumo en reposo (release, Windows 10)

Launcher (exe) 24 MB; WebView2 ≈ 280 MB de memoria total (165 MB privada) con 5 procesos tras limitarlo a un renderer, GPU en proceso y sin crashpad (antes ≈ 310 MB). CPU en reposo ≈ 0. Ejecutable de 17,4 MB (LTO + strip), MSI de 6,5 MB. Ordenes IPC de lectura: 4–6 ms.

## Rendimiento (Windows 10, release)

| Medicion | Resultado |
|---|---|
| Verificar 2000 assets de 64 KB (≈125 MB) | ≈4,5–6 s (dominado por apertura de archivos; hash en memoria: 373 MB/s) |
| Segundo arranque de una instancia ya instalada (sin red) | 0,4–1,2 s hasta `Running` |
| Instalacion limpia Vanilla 1.0 + Java 8 + natives | ≈35–60 s (red) |

## Accesibilidad

Auditada en navegador: objetivos ≥ 40 px (radios con etiqueta de 40 px), campos con etiqueta, un `<main>`/`<h1>`, foco movido al titulo al navegar, `aria-live` en estados, dialogos modales nativos, `prefers-reduced-motion`. El lienzo del editor tambien se maneja con teclado (flechas + Intro/Espacio); falta revisarlo con un lector de pantalla.
Tras la reforma visual no se repitio la auditoria completa (contrastes, tamaños): las etiquetas de 8-9 px heredadas de la maqueta se subieron a 10 px (solo se mantiene el 9 px de la marca «LAUNCHER»); los contrastes de los tres temas se midieron con la formula WCAG (2026-10-17): texto y estados ≥ 4,5:1 en todos; se oscurecieron `border-strong` (negro 2,18→3,2:1; claro 2,53→3,7:1) y `text-secondary` del claro (→ 4,8:1 sobre superficies elevadas). Falta revisar con un lector de pantalla.

## Pendiente de verificar antes de publicar

1. Recorrer en la ventana nativa los flujos que escriben datos (crear perfil, instalar): ya se comprobo el arranque, la lectura y jugar/cerrar una instancia existente con su consola.
2. Repetir el E2E en Linux y macOS (instalacion de Java con `tar.gz`, natives, rutas).
3. Microsoft contra servidores reales con una app registrada.
4. Instaladores firmados y actualizador (ver `docs/RELEASE.md`).
