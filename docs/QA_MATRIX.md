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
| Mods + Modrinth | ✅ resolver, locales, updates, lectura de Forge/NeoForge | ✅ (Fabric) busqueda, instalacion, actualizar, volver atras; ⚠️ Forge/NeoForge sin prueba real | |
| Modpacks `.mrpack` | ✅ import/export/rollback | ✅ export+import con referencias Modrinth | |
| Temas | ✅ validacion y almacen (incluye el tema azul oscuro) | 🖐 aplicacion en UI (negro y azul) | |
| Skins + editor | ✅ backend, 18 tests de logica del editor | 🖐 editor, 3D, biblioteca; ⚠️ teclado del lienzo sin probar a mano | Skin local no se ve en el juego sin mod |
| Interfaz reformada (maqueta) | ✅ `svelte-check`, build | 🖐 todas las vistas en navegador (820x560 y 1100x720, negro y azul); ⚠️ consola real del juego sin ver en ventana nativa | Ver `docs/UI_MIGRATION.md` |
| Descargas con redireccion | ✅ revalida https y host | — | |
| Recuperacion de datos | ✅ migracion, cuarentena | — | |

## Pruebas

- Rust: **257 pruebas pasan**, 0 fallan, 11 `#[ignore]` (requieren internet/ventana, se ejecutan a mano).
- Frontend: 18 pruebas (Vitest) de la logica del editor; `svelte-check` sin errores.
- Pruebas reales (manuales): `real_vanilla_install_launch_and_stop`, `real_fabric_install_launch_and_stop`, `real_quilt_install_launch_and_stop`, `real_fabric_with_modrinth_mod_loads_the_mod`, `real_forge_install_launch_and_stop`, `real_neoforge_install_launch_and_stop`, `real_fabric_install`, `real_quilt_install`, `real_modpack_export_and_import_use_modrinth_references`, `real_manifest_…`, y las de Modrinth en `crates/mods/tests/real_modrinth.rs`.

## Rendimiento (Windows 10, release)

| Medicion | Resultado |
|---|---|
| Verificar 2000 assets de 64 KB (≈125 MB) | ≈4,5–6 s (dominado por apertura de archivos; hash en memoria: 373 MB/s) |
| Segundo arranque de una instancia ya instalada (sin red) | 0,4–1,2 s hasta `Running` |
| Instalacion limpia Vanilla 1.0 + Java 8 + natives | ≈35–60 s (red) |

## Accesibilidad

Auditada en navegador: objetivos ≥ 40 px (radios con etiqueta de 40 px), campos con etiqueta, un `<main>`/`<h1>`, foco movido al titulo al navegar, `aria-live` en estados, dialogos modales nativos, `prefers-reduced-motion`. El lienzo del editor tambien se maneja con teclado (flechas + Intro/Espacio); falta revisarlo con un lector de pantalla.
Tras la reforma visual no se repitio la auditoria completa (contrastes, tamaños): hay etiquetas de 9-10 px heredadas de la maqueta que conviene revisar contra AA.

## Pendiente de verificar antes de publicar

1. Recorrer en la ventana nativa los flujos con datos (crear perfil, instalar, jugar): solo se verifico el arranque y el IPC de lectura para no escribir en los datos del usuario.
2. Repetir el E2E en Linux y macOS (instalacion de Java con `tar.gz`, natives, rutas).
3. Microsoft contra servidores reales con una app registrada.
4. Instaladores firmados y actualizador (ver `docs/RELEASE.md`).
