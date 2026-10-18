# Progreso del proyecto

Estado a 2026-10-17. Fuente de verdad del avance; el plan original esta en `PLAN_TECNICO.md` y `spec/13_LONG_SPRINTS.md`.
Leyenda: ✅ hecho y verificado · 🟡 hecho, verificacion parcial · ⛔ bloqueado por algo externo · ⬜ pendiente.

## Reforma visual (maqueta `DisorderLauncherUI`)

✅ Completada el 2026-10-17. La interfaz de `apps/desktop` sustituye al diseno anterior por el de la maqueta `DisorderLauncherUI/` (barra lateral, centro de sesion con consola real, Mis instancias, Explorar, Herramientas, Perfiles, Skins, Ajustes, tema azul oscuro incluido). Reglas, mapa y pendientes en [`UI_MIGRATION.md`](UI_MIGRATION.md): **leelo antes de tocar `apps/desktop/src`**. No cambia contratos IPC ni logica de negocio.

## Resumen

| Sprint | Contenido | Estado |
|---|---|---|
| 0 | Workspace, contratos de identidad, almacenamiento, credential store, Tauri + Svelte, tokens CSS, CI | ✅ |
| 1 | Offline Gaming + Vanilla end-to-end (manifiesto, descargas, natives, Java, lanzamiento, reparacion, IPC, UI) | ✅ |
| 2 | CRUD de instancias, RAM automatica, ajustes, dialogos de error, accesibilidad | ✅ |
| 3 | Fabric, mods locales, Modrinth (busqueda, dependencias, actualizar, volver atras) | ✅ |
| 4 | Quilt, Forge y NeoForge (instalador oficial, ADR-002) | ✅ |
| 5 | Modpacks `.mrpack` (importar con rollback, exportar) | ✅ |
| 6 | Microsoft (PKCE, Xbox/XSTS/Minecraft, refresh, almacen seguro) | 🟡 solo contra servidor falso |
| 7 | Temas JSON validados en Rust | ✅ |
| 8 | Biblioteca de skins locales | ✅ |
| 9 | Editor de skins (herramientas, capas, espejo, undo/redo, vista 3D) | ✅ |
| 10 | Seguridad, migraciones, recuperacion, QA, empaquetado MSI | 🟡 sin actualizador ni instaladores firmados |

## Lo implementado, por area

**Identidad (`identity-api/-local/-microsoft`, `credential-store`, `account-storage`)**: `IdentityProvider` desde el primer sprint; perfiles locales con UUID `OfflinePlayer:`; `Instance` solo guarda `profile_id`; ningun token en JSON ni frontend; `FakeMicrosoftProvider`; cuentas con `schemaVersion` y migracion v0→v1 con copia `.bak-v0`; archivos danados apartados (`.corrupt`); ids validados contra path traversal.

**Motor (`minecraft-meta/-install/-launch`, `java-runtime`, `download-manager`, `diagnostics`)**: descriptores y reglas (os/arch/features, `arm64`); `InstallPlan` con natives extraidos de forma segura, assets legacy (`virtual`, `map_to_resources`) y hashes validados; Java Adoptium con SHA-256 y seleccion estricta (Java 8 exacto); descargas atomicas con allowlist de dominios Mojang; `LaunchPlanBuilder` (placeholders centralizados, overlays de loader, args antiguos y modernos) y `ProcessRunner` sin shell, entorno por allowlist/prefijos, redaccion de secretos; verificacion y reparacion.

**Instancias y aplicacion (`instance-manager`, `launcher-core`)**: `instance.json` versionado, ids y versiones validados; `Launcher` orquesta cuentas, instalacion bajo demanda, Java, loaders, mods, modpacks, temas, skins, Microsoft y ajustes sin depender de Tauri; errores en espanol con `ErrorKind` (offline/reparable/inicio de sesion).

**Loaders (`loaders`)**: Fabric y Quilt (meta API, hosts de confianza), Forge y NeoForge (instalador oficial verificado por `.sha1`, ejecutado con argv, timeout y entorno reducido).

**Mods y modpacks (`mods`, `modpacks`)**: lectura de `fabric.mod.json`, activar/desactivar, dependencias faltantes en lenguaje humano, Modrinth con CDN de confianza y hashes, resolver con ciclos, actualizar/rollback; `.mrpack` con rutas y hosts validados, overrides sin poder pisar `.disorder/`, exportacion sin mundos ni logs.

**Microsoft (`identity-microsoft`)**: PKCE S256, `state`, callback loopback de un solo uso, cadena Xbox→XSTS→Minecraft, ownership, refresh con candado por cuenta y rotacion de token, errores transitorios sin borrar tokens, deshabilitado sin client id.

**Temas y skins (`themes`, `skins`)**: validacion estricta (colores, fuentes, rangos, contraste AA); PNG real, 64x64/64x32, ≤ 256 KB, recodificado sin metadatos; editor con logica pura probada (Vitest).

**Aplicacion de escritorio (`apps/desktop`)**: Tauri 2 + Svelte 5; pantallas de bienvenida, inicio, instancias, mods, perfiles, skins, editor, ajustes; CSP estricta; solo `core:default`; dialogos nativos invocados desde Rust.

## Verificacion realizada

- 267 pruebas Rust + 18 de frontend; `clippy -D warnings` y `svelte-check` limpios.
- Ejecucion real en Windows 10: Vanilla 1.0, Fabric, Quilt, Forge 1.14.4 y NeoForge 1.21.1 arrancan en el juego; Fabric API instalada desde Modrinth se carga; export/import `.mrpack` con referencias Modrinth.
- Empaquetado: MSI de 6 MiB; `.exe` release verificado por CDP (UI bajo CSP, IPC real, 0 errores); `tauri dev` abre la ventana y lanza Minecraft.
- Detalle y cifras en `QA_MATRIX.md`; hallazgos de seguridad en `SECURITY_AUDIT.md`.

## Bugs reales encontrados al ejecutar (todos corregidos)

Java ≥ 9 aceptado para versiones de Java 8 · runtimes gestionados redescargados en cada arranque · assets legacy sin materializar · `powershell Expand-Archive` con rutas interpoladas · path traversal por id de cuenta y por version de Minecraft · hashes de assets sin validar · CSP en `null` · un perfil corrupto bloqueaba todos · reglas ARM con nombre equivocado · variables de entorno graficas perdidas en Linux · panico al iniciar en `tauri dev` (dos loggers globales).

## Pendientes

### Bloqueados por algo externo (no se simulan)
- ⛔ **Microsoft real**: requiere registrar una app de Azure (cliente publico, redirect `http://localhost`, scope `XboxLive.signin offline_access`) y su client id. Minecraft Services puede rechazar apps no aprobadas; en ese caso se informa y Offline Gaming sigue.
- ⛔ **Actualizador automatico**: requiere par de claves de firma y endpoint HTTPS con el manifiesto (`RELEASE.md`).
- ⛔ **Instaladores firmados**: certificado Authenticode (Windows) y cuenta de Apple Developer (macOS).

### Verificacion pendiente
- ⬜ **Linux y macOS**: CI de Linux configurado sin ejecutar; macOS sin CI; ningun E2E fuera de Windows (Java `tar.gz`, natives, rutas, ARM).
- ⬜ **Recorrido completo en la ventana nativa con datos** documentado paso a paso (solo se verifico arranque, IPC de lectura y un lanzamiento).
- ⬜ **`cargo audit`**: anadido a CI como job `audit`, sin primer resultado.
- ⬜ **Empaquetado NSIS** fallo en este equipo (`os error 32` en la carpeta de herramientas de Tauri); solo MSI verificado.

### Mejoras conocidas
- ✅ Redirecciones: tras seguir un redirect se vuelve a exigir https y host permitido (`download-manager`); el hash sigue protegiendo la integridad.
- 🟡 Mods de Forge/NeoForge: ahora se listan (lee `mods.toml`/`neoforge.mods.toml`), activan/desactivan, quitan y se buscan en Modrinth con su categoria; probado con jars sinteticos, falta una prueba real con Modrinth y el juego (los E2E reales de Forge/NeoForge no incluyen mods).
- ⬜ Skins locales: se guardan y previsualizan; verlas dentro del juego exige un mod compatible.
- 🟡 Editor de skins: alternativa de teclado hecha (flechas mueven el cursor, Mayús salta 8, Intro/Espacio pintan; probada solo por compilacion, falta revisarla a mano).
- ✅ Tema: `typography.scale` x `density.uiScale` se aplican como zoom; ⬜ `density.compact` aun no.
- ⬜ Rendimiento: verificar ~2000 assets tarda ≈ 4,5–6 s (E/S de archivos en Windows).
- ⬜ `style-src 'unsafe-inline'` sigue necesario por los estilos de Svelte.
- ⬜ Rama de trabajo: el repo usa `master`; el plan menciona `main`.
