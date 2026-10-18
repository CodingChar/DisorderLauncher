# Reforma visual: maqueta `DisorderLauncherUI` → `apps/desktop`

> **Si eres otro chat/agente: lee este archivo antes de tocar `apps/desktop/src`.**
> Estado: **reforma completada** (2026-10-17). Estas reglas siguen vigentes para cualquier cambio de interfaz posterior.

## Que es esto

`DisorderLauncherUI/` (en la raiz del repo) es la **maqueta visual** aprobada por el usuario: Svelte 5 + CSS propio, con
datos simulados. Define el aspecto y la distribucion nuevos del launcher. La reforma consiste en **sustituir la
parte visual de `apps/desktop`** por la de la maqueta **sin perder ninguna funcion real** (IPC, stores, seguridad).

La maqueta es solo referencia visual: **no se ejecuta ni se importa desde la app**, y no se copia su logica simulada
(sesiones demo, logs falsos, catalogo de ejemplo). Su `AGENTS.md` describe su propio repo; para la app real manda este documento.

## Reglas durante y despues de la reforma

1. **Logica intacta**: `lib/api`, `lib/stores/*` (salvo lo anotado abajo), `src-tauri` y los crates no se rediseñan por motivos esteticos.
2. **Sistema visual unico**: las clases globales de `apps/desktop/src/app.css` (`.panel`, `.primary`, `.secondary`, `.pill`,
   `.page-heading`, `.library-*`, `.settings-panel`, `.account-row`, `.session-*`, `.console-*`, `.modal`, ...) son la fuente de verdad.
   No inventes otro estilo por pantalla; reutiliza esas clases y los componentes de `lib/components`.
3. **Tokens, no colores fijos**: usa `--bg --surface --surface-alt --surface-raised --text-primary --text-secondary --border --border-strong --action --action-text`
   y `--r-sm/md/lg/xl` (los temas JSON los sobrescriben). La unica excepcion son los verdes de estado (`--ok`).
4. **Sin recursos remotos**: nada de Google Fonts ni CDNs (la CSP es `default-src 'self'` y el launcher funciona sin internet). La maqueta importa fuentes remotas; en la app NO.
5. **Texto para personas no tecnicas** (regla de `CLAUDE.md`): nada de terminos internos fuera de la pantalla avanzada. Lo que la maqueta marca como «solo idea» se muestra como «Proximamente», nunca como funcion que no existe.
6. **Nada simulado en produccion**: la consola muestra la salida real del juego (`launch://stdout|stderr`), las herramientas ejecutan acciones reales o estan deshabilitadas con «Proximamente».
7. Accesibilidad AA, foco visible, objetivos >= 40 px, `prefers-reduced-motion` (spec 08).
8. Tras cualquier cambio de UI: `npm run check`, `npm test`, `npm run build` en `apps/desktop` y revision en el navegador (`npm run dev` usa el backend simulado de `lib/api/mock.ts`).

## Mapa maqueta → app real

| Maqueta | Vista (`nav.view`) | Fuente real |
|---|---|---|
| Inicio / Centro de sesion | `home` | instancia seleccionada + `sessions` (estado, progreso, consola real) |
| Mis instancias (biblioteca) | `instances` | `instances.list`; Jugar / Configurar |
| Editor de instancia | `create` / `edit` | `CreateInstance` / `EditInstance` (campos reales, no los de la maqueta) |
| Explorar | `explore` | atajos reales: crear vanilla, con mods, importar `.mrpack` (sin catalogo falso) |
| Herramientas | `tools` | reparar, exportar/importar modpack reales; el resto «Proximamente» |
| Perfiles | `accounts` | `accounts` (Offline Gaming + Microsoft) |
| Skins | `skins` / `skinEditor` | `skins` (biblioteca real y editor) |
| Ajustes | `settings` | `settings` (tema, descargas, Java, temas JSON) |
| Boton luna (tema azul) | cabecera | tema incluido `disorderlaucher.default.blue` (Rust `themes`) |
| Modal Ayuda / Perfil | `lib/components/Modal` | texto de ayuda real; el perfil lleva a `accounts` |

Fuera de la maqueta y conservado: pantalla de bienvenida, `Mods`, `SkinEditor`, avisos de arranque, `ConfirmDialog`.
Estos reciben el nuevo aspecto por los componentes compartidos y la cabecera `PageHeading`.

## Estado

| Fase | Contenido | Estado |
|---|---|---|
| 0 | Analisis, este documento, reglas en `CLAUDE.md`/spec 08 | ✅ |
| 1 | Sistema visual global (`app.css`), tema azul incluido, temas con radios de la maqueta | ✅ |
| 2 | Componentes compartidos (Button, Card, Input, Select, ConfirmDialog, InstanceCard, ProfileBadge, PageHeading, Icon, ItemLogo) | ✅ |
| 3 | Shell: barra lateral, cabecera, pie, ayuda, navegacion con vistas nuevas | ✅ |
| 4 | Pantallas: Inicio, Mis instancias, Explorar, Herramientas | ✅ |
| 5 | Pantallas: Perfiles, Skins, Ajustes, Crear/Editar, Mods, Editor de skins, Bienvenida | ✅ |
| 6 | Verificacion y documentacion final | ✅ (ventana nativa: ver pendientes) |

Leyenda: ✅ hecho · 🟡 en curso · ⬜ pendiente. El que avance una fase actualiza esta tabla **en el mismo commit**.

## Decisiones que conviene recordar

- `Instance` sigue referenciando `profile_id`; la barra lateral y la cabecera muestran el perfil predeterminado (`accounts.current`), nunca tokens.
- El selector de icono de instancia de la maqueta (bloques/objetos) **no se porto**: el modelo real de `Instance` no guarda icono. Los iconos se derivan del cargador (`lib/components/ItemLogo.svelte`). Guardarlo seria un cambio de contrato (`docs/IPC.md`, `instance.json`) fuera de esta reforma.
- Las skins de la maqueta (colores por parte del cuerpo) se sustituyen por las reales (PNG + vista 3D).
- La RAM/Java por instancia de la maqueta se mapean a `memory`/`java` reales (`auto` | fijo/ruta).

## Lo que cambio (referencia rapida)

- `apps/desktop/src/app.css`: sistema visual completo (sin fuentes remotas), tokens + estados `--ok/--warn/--danger`.
- Componentes nuevos: `Icon`, `Avatar`, `ItemLogo` (icono por cargador), `PageHeading`, `HelpDialog`. Reescritos sobre clases globales: `Button`, `Card`, `Input`, `Select`, `ProgressBar`, `ConfirmDialog`, `InstanceCard`, `ProfileBadge`.
- Vistas nuevas: `Instances` (biblioteca), `Explore`, `Tools`; `Home` es ahora el «Centro de sesion» con consola real.
- `nav.svelte.ts`: vistas `instances|explore|tools`, `viewLabel` (migas), `navParent` (entrada resaltada), `nav.create(loader)`.
- `sessions.svelte.ts`: consola por instancia (`logOf`, `clearLog`, maximo 1000 lineas) alimentada por `launch://stdout|stderr`; hora de inicio de la sesion.
- `api`: `events.onGameOutput`. Sin cambios de comandos IPC.
- Rust `themes`: tema incluido `disorderlaucher.default.blue`; radios de los temas incluidos 7/8/13/18 (antes 8/10/18/24).
- `.claude/launch.json`: servidor de desarrollo `desktop-ui` (navegador con backend simulado).

## Pendiente conocido de la reforma

- **Ventana nativa**: verificada el 2026-10-17 con `tauri dev` y backend real: arranca bajo la CSP de desarrollo sin errores, Jugar lanza la instancia existente, la consola muestra la salida real del juego y «Cerrar juego» lo detiene. Sigue sin recorrerse a mano la creacion de perfiles/instancias nuevas.

- Crear instancia ya incluye «Opciones avanzadas» (memoria y Java), igual que la maqueta.
- Icono de instancia elegible: hecho (12 iconos propios en pixel art, `itemIcons.ts`; campo `icon` en la instancia).
- `Mods` y `SkinEditor` solo recibieron el aspecto por los componentes compartidos y la cabecera; no se rediseño su disposicion interna.
- Fuentes Space Grotesk e IBM Plex Sans incluidas en `apps/desktop/public/fonts` (woff2 latin, licencia OFL, via @font-face en `app.css`); verificado en la ventana nativa bajo la CSP: se cargan sin errores.
- `typography.scale` x `density.uiScale` de los temas se aplican como `zoom` del documento (`theme.ts`, con prueba). `density.compact` sigue sin aplicarse.
