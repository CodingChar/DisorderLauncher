# Guía de portado: interfaz Minecraft (`maqueta`) → `DisorderLaucher/apps/desktop`

Destino: `C:\Users\Pollito\Desktop\developments\DisorderLaucher\apps\desktop` (Tauri 2 + Svelte 5 **runes** + TypeScript, ~6 700 líneas de frontend).
Origen visual: `C:\Users\Pollito\Desktop\maqueta` (Svelte 5 sintaxis *legacy* + JS, Vite 6, solo datos simulados).

Objetivo: sustituir **solo la capa visual** de la app real por el estilo de menús de Minecraft 1.7.10 de la maqueta, **sin perder ninguna función real** (IPC, stores, seguridad, temas, skins, mods).

Índice: 0 Lectura previa · 1 Decisiones · 2 Qué copiar · 3 Infraestructura · 4 Sistema de coordenadas y plantillas · 5 Componentes que faltan (especificación) · 6 Pantallas (especificación) · 7 Stores/temas · 8 Limpieza · 9 Verificación · 10 Conflictos y licencia · 11 Recomendaciones.

---

## 0. Lee primero (obligatorio)

En el destino lee, en este orden: `.claude/CLAUDE.md`, `docs/UI_MIGRATION.md`, `docs/spec/08_VISUAL_DESIGN_SYSTEM.md`, `docs/spec/09_THEME_SYSTEM.md`, `docs/IPC.md`.
`UI_MIGRATION.md` describe una reforma anterior (maqueta `DisorderLauncherUI`, estilo moderno). **Esta migración la reemplaza**: al terminar, crea `docs/UI_MIGRATION_MINECRAFT.md`, enlázalo desde el README y marca como obsoletas las reglas del anterior que contradigan esta guía (§10).

## 1. Decisiones de arquitectura (no las discutas, aplícalas)

1. **No copies `App.svelte` de la maqueta.** Tiene datos y lógica simulados. Copia solo presentación: componentes `Mc*`, `BitmapText`, `font`, `logo`, estilos y assets.
2. **Lógica intacta.** No toques `src/lib/api`, `src/lib/stores/*` (salvo §7), `src-tauri/` ni `crates/`. Las pantallas reales (`src/routes/*.svelte`) conservan sus `$state/$derived`, llamadas a `api.*`/stores, validaciones y manejo de errores; solo se reescribe su *template* y su CSS.
3. **Runes en el destino.** La maqueta usa `export let`, `$:`, `on:click`. Convierte: `$props()`, `$derived`, `$state`, `$bindable()`, `onclick=`. Componentes con `<script lang="ts">`; `font.js` y `logo.js` pasan a `.ts` con tipos.
4. **Escenario fijo en "píxeles GUI de Minecraft"** (§4): coordenadas enteras, posiciones absolutas, escalado entero con `transform: scale(var(--s))`. No mezcles flex/grid fluido dentro del stage: es lo que garantiza alineación sin overflow.
5. **Todo texto visible = canvas con fuente bitmap** (`BitmapText`). Nada de texto HTML visible (solo `<input>` invisibles y `aria-label`). Todo texto lleva ancho máximo y se acorta con `fitText` ("...") o se parte con `wrap` (§5.1).
6. **Sin recursos remotos** (CSP `default-src 'self'`): assets en `public/assets/`, rutas `/assets/...`. No se copian las `.woff2` de la maqueta (no se usan).
7. **Licencia:** el launcher será de código libre; los assets de Minecraft se usan tal cual. Añade una nota de atribución (README → "Créditos": «Texturas y fuente inspiradas en Minecraft, © Mojang; DisorderLauncher no está afiliado a Mojang ni Microsoft») y nada más.

## 2. Qué copiar de la maqueta

| Origen (`maqueta/`) | Destino (`apps/desktop/`) | Notas |
|---|---|---|
| `assets/ascii.png`, `button.png`, `button-hover.png`, `widgets.png`, `options_background.png`, `world_selection.png`, `unknown_server.png`, `panorama/` (6 PNG), `splashes.txt` | `public/assets/` | Mismas rutas relativas; el CSS usa `/assets/...`. |
| `src/lib/font.js` | `src/lib/mc/font.ts` | Anchos, `renderText`, `stringWidth`, `fitText`, `tailText`, `fontReady`. **No cambies** la tabla de caracteres extra (glifos 128+). |
| `src/lib/BitmapText.svelte`, `McLabel`, `McButton`, `McField`, `McSlider` | `src/lib/mc/*.svelte` | Portar a runes (§5). |
| `src/lib/logo.js` | `src/lib/mc/logo.ts` | Logo en letras bloque + splash. |
| `src/lib/PixelIcon.svelte` | solo referencia | En el destino los iconos son PNG (`src/assets/instance-icons`, `itemIcons.ts`). |
| `styles.css` | `src/mc/stage.css` | Stage, panorama 3D CSS, logo, splash, `.mc-button`, `.mc-language`, footer, `.sr-only`. |
| `src/svelte.css` | `src/mc/screens.css` | `.screen/.dirt`, labels, slider, field, bandas de lista, filas, consola. |

## 3. Fase 1 – Infraestructura (verificar antes de seguir)

1. `git checkout -b ui/minecraft-style`.
2. Copiar assets (§2); `npm run dev` debe servir `/assets/ascii.png` con 200.
3. Portar `font.ts` + `BitmapText.svelte`. Prueba: renderizar `Mis instancias ñ á é í ó ú ü ¿ ¡ « »` y ver que los acentos salen.
4. Crear `McStage.svelte`:
   - calcula `factor = max(1, floor(min(innerWidth·dpr / 320, innerHeight·dpr / 240)))` (píxeles de dispositivo) y expone variables CSS `--s` (escala CSS = factor/dpr), `--W`, `--H` (= tamaño de ventana en px GUI, enteros), `--cx` (= W>>1); recalcula en `resize` y al cambiar `devicePixelRatio`;
   - pinta el fondo: panorama 3D en Menú principal, `options_background.png` (tinte `--shade`) en las demás;
   - `<slot>` (en runes `{@render children()}`) para la pantalla activa y para el diálogo modal;
   - respeta `prefers-reduced-motion` y pausa el panorama con `visibilitychange`.
5. `src/main.ts`: importar `mc/stage.css` y `mc/screens.css`. `app.css` se retira solo al final (§8).
6. Tamaños de GUI que debes asumir (ventana Tauri mínima 820×560, defecto 1100×720; dpr=1):

| Ventana | Factor | Tamaño GUI (W×H) |
|---|---|---|
| 820×560 | 2 | 410×280 |
| 1100×720 | 3 | 366×240 |
| 1920×1080 | 4 | 480×270 |
| (teórico mínimo) | — | 320×240 |

**Todo layout debe funcionar para cualquier W ≥ 320 y H ≥ 240.** Diseña a 320×240 y deja que W/H extra solo recoloquen (centrado y bandas elásticas).

## 4. Sistema de coordenadas y plantillas de pantalla

- Origen: `x` se mide **desde el centro horizontal** (negativo = izquierda); `y` **desde arriba**, o desde abajo con `anchor="bottom"` (`y` = distancia al borde inferior). Todo entero.
- Alturas: botón 20, campo 20, slider 20, separación vertical estándar 24 entre filas de botones (botón + 4), título de pantalla centrado en `y=12` (blanco), etiquetas de campo 10 px sobre el control (gris `#a0a0a0`), texto de ayuda gris `#808080`.
- Anchos estándar: botón completo 200 (`x=-100`), medio 98 (`x=-100` / `x=2`), tercio 64 (`x=-100,-32,36` con ancho 64 → huecos de 4), listas 220–310.
- Colores: normal `#ffffff`, hover `#ffffa0`, deshabilitado `#a0a0a0`, secundario `#808080`, error `#ff5555`, éxito `#55ff55`, aviso `#ffaa00`. Sombra = color>>2 (la hace `renderText`).

**Plantilla A – lista (Mis instancias, Perfiles, Skins, Mods, versiones):**
```
y=12         título (centrado)
y=22  h=20   campo de búsqueda (si aplica) w=200 x=-100
y=32/44 → H-64   banda de lista (McList) ancho = min(310, W-16), centrada
H-52         fila de botones 1 (3 botones de 98/…: acción principal a la izquierda)
H-28         fila de botones 2 (… + "Atrás" o "Hecho" siempre abajo-derecha)
```
**Plantilla B – formulario corto (Crear, Editar, Ajustes, Perfil nuevo):** título y=12; controles en columna centrada w=200 empezando en y=32, paso 24 (campos con etiqueta: paso 34); Guardar/Cancelar en la última fila anclada abajo (`H-28`, 98+98). Si la columna supera `H-64` (a 240 de alto caben 7 controles), **divide en subpantalla** ("Opciones avanzadas…", como en Minecraft), nunca encojas.
**Plantilla C – diálogo:** `McDialog` (§5.5).

Regla de oro: una pantalla = ≤ 7 controles en columna a 240 de alto. Si hay más, subpantalla o `McScroll`.

## 5. Componentes que faltan: especificación detallada

Todos en `src/lib/mc/`, runes + TS, con `*.test.ts` (vitest, `environment: node`) para lógica pura (ajuste de texto, índices de scroll, paginación) — mueve la lógica a funciones puras en `mc/logic.ts` para poder probarla sin DOM.

### 5.0 Convenciones comunes
- Props posicionales: `x`, `y`, `w`, `h`, `anchor` ('top' por defecto | 'bottom'). Cada componente renderiza un elemento `position:absolute` con `left: calc(var(--cx) * 1px + x * 1px)`; y calcula `top` o `bottom` según `anchor`.
- Eventos como props de función: `onclick`, `onchange`, `onactivate`.
- `disabled` real (atributo nativo), nunca solo estilo.
- Foco visible: borde blanco de 1 px GUI alrededor del control (`:focus-visible`), distinto del hover amarillo.
- Los canvases de texto llevan `aria-hidden="true"`; el elemento nativo lleva `aria-label` con el texto completo (sin acortar).
- Texto vacío o undefined → renderiza cadena vacía, nunca "undefined".

### 5.1 `McText` (párrafo) + helpers `wrap` y `fitText`
- Props: `text`, `x`, `y`, `w` (ancho máximo), `color`, `align='left'|'center'`, `maxLines` (0 = sin límite), `lineHeight=10`.
- `wrap(text, w)` ya existe en la maqueta (divide por palabras con `stringWidth`). **Añade**: palabras más largas que `w` se cortan por carácter; respeta `\n`; si hay `maxLines` y sobra texto, la última línea se acorta con `fitText` y "...".
- Uso: hints (`hint`), notas, errores, ayuda, descripciones. Reemplaza `<p class="subtle">`, `.form-error`, `.form-ok`.
- Errores: `role="alert"` oculto con el texto, canvas en `#ff5555`. Éxitos: `role="status"`, `#55ff55`.
- Test: `wrap('a b c', 3 palabras)`, palabra gigante, `\n`, cadena vacía, `maxLines`.

### 5.2 `McCycle` (sustituye `Select`, radios y grupos pequeños)
- Props: `label`, `options: {value, label}[]`, `value` (`$bindable`), `x,y,w=200`, `disabled`, `onchange`.
- Aspecto: un `McButton` cuyo texto es `${label}: ${opción actual}`. Clic o Enter/Espacio/→ = siguiente (circular); clic derecho o ← = anterior. Cada cambio llama `onchange`.
- Accesible: `<button aria-label="{label}: {actual}. Pulsa para cambiar">`; no hace falta `aria-live`.
- Usar cuando hay **≤ 6 opciones cortas**: Tipo de instancia (vanilla/Fabric/Quilt/Forge/NeoForge), Perfil que juega, Tema (Sistema/Claro/Oscuro), Velocidad de descarga, Java (Automático/Ruta propia), Memoria (Automática/Elegir cantidad), Rendimiento (`PerformanceOptions.levels` → un cycle con el `label`; el `hint` de la opción actual se muestra debajo con `McText`), Modelo de skin (Clásico/Fino).
- Para listas largas (versiones, perfiles >6, temas JSON) usa `McList` (§5.3).
- Equivalente a `<input type="radio" bind:group>`: al migrar mantén las mismas variables de estado (`memoryMode`, `javaMode`, `theme`, `speed`) y los mismos valores; solo cambia el control.

### 5.3 `McList` (el componente crítico)
Props: `items: T[]`, `getId(item)`, `selectedId` (`$bindable`), `x,y,w,h` (banda), `rowHeight=36`, `row` (snippet runes `{#snippet row(item, selected, y)}` que pinta contenido con `BitmapText`/icono), `onactivate(item)` (doble clic / Enter), `empty` (texto cuando no hay filas), `loading` (bool), `ariaLabel`.

Comportamiento (copiar de `GuiSlot` de Minecraft 1.7.10):
- Fondo de banda: `options_background` oscurecido (rgba 0,0,0,.6) con degradados de 4 px arriba/abajo; fila seleccionada: borde externo `#808080` y relleno interior `#000000` (1 px cada uno).
- Layout de fila (36 de alto): icono 32×32 en `(x+2, y+2)`; línea 1 (nombre, blanco) en `(x+32+5, y+2)`; línea 2 (`#808080`) en `y+13`; línea 3 (`#808080`) en `y+23`. Ancho de texto máximo = `w - 32 - 5 - 8 (scrollbar+margen)`, siempre con `fitText`.
- Scroll: rueda = 18 px por muesca (rowHeight/2) con rebote en límites; barra de 6 px a la derecha, pulgar mínimo 32 px, arrastrable; solo aparece si `content > h`. Cada posición de scroll es un **entero**.
- Teclado: ↑/↓ (±1 fila), RePág/AvPág (±filas visibles), Inicio/Fin, Enter = `onactivate`, Esc no hace nada (lo gestiona la pantalla). Al mover la selección, scroll automático para que la fila quede entera en pantalla.
- Ratón: clic selecciona, doble clic `onactivate`, arrastre en la barra.
- **Virtualización**: pintar solo filas visibles ± 2; cachear los canvases de texto por `id+text`. Obligatorio para la lista de versiones (cientos/miles).
- Estados: `loading` → texto centrado "Cargando..." gris; vacía → `empty` centrado (acortado/partido con `McText`); error → la pantalla muestra `McText` en rojo sobre la banda (no es de `McList`).
- A11y: contenedor `role="listbox"` (focusable, `aria-label`, `aria-activedescendant`); cada fila visible tiene un `<div role="option" id aria-selected aria-posinset aria-setsize class="sr-only">` con el texto completo.
- Tests (`logic.ts`): dado `n, rowHeight, h, scroll` → rango visible; clamp de scroll; `ensureVisible(index)`; tamaño/posición del pulgar; mapeo wheel→scroll.

### 5.4 `McProgress` (sustituye `ProgressBar`)
- Props: `percent: number | null`, `label`, `x,y,w=200`.
- Aspecto: alto 10; marco 1 px `#000` + interior `#808080` oscuro; relleno `#55ff55` hasta `floor(percent/100 · (w-2))`; el texto `label` (blanco) va **encima** de la barra a `y-11`, y el porcentaje (`NN%`) centrado dentro.
- `percent === null` (indeterminado): segmento de 40 px que se desplaza ida y vuelta (CSS, 1.2 s); con `prefers-reduced-motion` queda estático al 50 % y el texto añade "..."
- A11y: `role="progressbar" aria-valuemin=0 aria-valuemax=100 aria-valuenow aria-label={label}` (sin `valuenow` si es indeterminado).
- Usos reales: lanzar (fases `InstallPhase` + `completed/total`), reparar, importar modpack (`instances.importPercent`), descargar mods.

### 5.5 `McDialog` (sustituye `ConfirmDialog`, `HelpDialog`, `Modal`)
- Props: `title`, `message` (string o líneas), `buttons: {label, onclick, primary?, danger?}[]` (1–3), `onclose` (Esc), `cancelIndex` (botón que se enfoca primero y que responde a Esc; para acciones destructivas es el "Cancelar").
- Aspecto: capa a pantalla completa `rgba(0,0,0,.6)` sobre la pantalla actual; título centrado y=`H/2-40` (blanco), mensaje con `McText` centrado w=280 desde `H/2-24`, botones anclados a `H/2+24`, w=98 si son 2 (x=-100 y 2), 200 si es 1.
- Comportamiento: al abrir, el resto del stage recibe `inert`; foco al botón `cancelIndex`; Tab circula solo entre los botones (trap); Esc = `onclose`; al cerrar, el foco vuelve al elemento que lo abrió.
- A11y: `role="dialog" aria-modal="true" aria-labelledby aria-describedby`.
- Usos: confirmar eliminar instancia/skin/perfil (`toRemove`), ayuda (texto real de `HelpDialog`, sin botones salvo "Hecho"), confirmar cerrar el juego si procede, errores bloqueantes.

### 5.6 `McToggle`
- Es un `McCycle` fijo con `Sí`/`No` (`Rápido: Sí`). Props: `label`, `checked` (`$bindable`). `role="switch" aria-checked`.
- Reemplaza todos los `<input type="checkbox">`: filtros de versiones, "Copiar datos", capas/espejo/cuadrícula del editor de skins, interruptor de activar/desactivar de cada mod.

### 5.7 `McTabs`
- Props: `tabs: {id,label}[]`, `value` (`$bindable`), `x,y,w`. Tabs repartidas en ancho igual (2 → 98/98 con hueco 4; 3 → 64).
- La pestaña activa se dibuja con el sprite de deshabilitado pero texto **blanco** (`#fff`); las otras como botones normales. `role="tablist"`, botones `role="tab" aria-selected`; ←/→ cambian de pestaña.
- Usos: Versiones oficiales/Personalizado (CreateInstance), Mods instalados/Buscar mods.

### 5.8 `McViewport` (para contenido no bitmap: 3D, canvas de skin, vista previa)
- Props: `x,y,w,h`, marco opcional (1 px `#000` + fondo `rgba(0,0,0,.5)`).
- Contiene un `<canvas>` cuyo **buffer interno mide `w·s × h·s`** (s = factor del stage, leído de `--s`; actualiza con `ResizeObserver`) y cuyo tamaño CSS es `w × h` en px GUI, con `image-rendering: pixelated`. Así `SkinPreview3D` y el lienzo del editor se ven nítidos bajo el `transform: scale`.
- Se conserva la lógica interna de `SkinPreview3D.svelte`, `SkinPreview.svelte` y de `skinGeometry.ts`: solo se les pasa el canvas/tamaño. Pausar `requestAnimationFrame` si la pantalla no está visible.

### 5.9 `McScroll` (solo si no hay alternativa)
- Contenedor vertical con la misma barra que `McList`; `focusin` en un hijo hace scroll para dejarlo entero visible; la rueda mueve ±18. Se usa en: detalle de Mods (resultados con descripción) y Editor de skins (panel lateral de opciones). **No** en Ajustes/Crear/Editar (usan subpantallas).

### 5.10 Ajustes a los componentes existentes
- `McField`: añadir props `type='text'|'password'` (en password el texto bitmap muestra `*`), `onenter`, `error` (borde rojo `#ff5555` + `McText` debajo), `hint` (`McText` gris debajo), `autofocus`, `disabled`. El `<input>` real invisible lleva `aria-label`, `aria-invalid`, `aria-describedby`. Cursor `_` parpadea solo con foco. Selección con ratón/doble clic: debe funcionar (el input real la maneja; el overlay bitmap muestra el texto desplazado para que **siempre se vea el final** cuando no cabe: `tailText`).
- `McSlider`: para RAM, `min=1,max=16 (GB)`, texto "Memoria máxima: 4 GB". Añadir `step` y `format(value)`.
- `McButton`: añadir `variant='normal'|'primary'` (primary = mismo sprite; solo se usa para el orden de foco y `type="submit"`), `type` (`button|submit`) y `form` para que Enter envíe formularios como hoy (`<form onsubmit>` real invisible que contiene los campos y el botón de enviar).

## 6. Pantallas: especificación pantalla por pantalla

La navegación real es `nav.view` (`src/lib/stores/nav.svelte.ts`) y **no se cambia**. Quita la barra lateral/cabecera: el menú principal + botón "Atrás/Hecho" de cada pantalla la reemplazan. `Atrás` = `nav.go(navParent[nav.view])` (misma tabla que ya existe). `Esc` = Atrás en todas las pantallas (excepto cuando hay `McDialog`, donde cierra el diálogo).

Para cada pantalla se indican: **vista · fuente de datos · controles (x, y, w) · estados · acciones/handlers a conservar**.

### 6.1 Menú principal (`App.svelte` shell, sin `nav.view` propio: es `home`/raíz)
- Fondo panorama, logo, splash (`splashes.txt`), pie "DisorderLauncher 0.1.0" (usa `APP_VERSION` de `lib/version`) a la izquierda y copyright a la derecha (`max = W - 126`).
- Botones (w=200, x=-100, `y` desde `H/4+48`, paso 24): **Mis instancias** (`nav.go('instances')`), **Explorar** (`explore`), **Herramientas** (`tools`), **Perfiles y skins** (`accounts`; ver nota), 1 fila de dos botones 98: **Ajustes...** (`settings`) / **Salir**. Botón de idioma (icono) → abre `McDialog` de ayuda (`helpOpen`).
- Nota "Perfiles y skins": el menú tiene un solo botón; la pantalla `accounts` incluye un botón "Skins" que hace `nav.go('skins')`, y `skins` tiene "Perfiles".
- Qué es "home" ahora: `nav.view === 'home'` conserva su significado de **Centro de sesión** (6.3). El menú principal es la pantalla raíz cuando no hay sesión activa; define en `nav` una vista nueva `'menu'` (único cambio permitido en `nav.svelte.ts`: añadirla a `View`, `viewLabel`, `navParent`) y haz que sea el valor inicial. `Jugar` y `Importar` siguen yendo a `home` como hoy.
- Conserva: avisos (`notices`) → `McDialog` o línea roja bajo los botones; estado `online` → línea gris "Sin conexión"; `themeError` → línea roja; pantalla de bienvenida: si `needsProfile` se muestra 6.9 en lugar del menú.
- "Salir": `import { getCurrentWindow } from '@tauri-apps/api/window'; getCurrentWindow().close()`; en el navegador de desarrollo no hace nada (botón deshabilitado si `!('__TAURI_INTERNALS__' in window)`). Comprueba que `tauri.conf.json`/capabilities permiten `core:window:allow-close`; si no, añádelo en `src-tauri/capabilities/*.json` (único cambio permitido allí).

### 6.2 Mis instancias (`instances`) — plantilla A
- Datos: `instances.list`, `instances.selected`, `instances.select(id)`, `sessions.get(id)?.phase`, `sessions.isBusy(id)`.
- Lista: `McList` (`getId = i.id`, `selectedId` ↔ `instances.selectedId`). Fila: icono real (resolver con `itemIcons.ts`; si `icon===null`, el del loader) 32×32 pixelated; línea 1 = nombre; línea 2 = `${minecraftVersion} · ${loaderLabel}` (+ " · En marcha" en `#55ff55` si running); línea 3 = perfil (`accounts.list.find(profileId)?.label`).
- Botones: fila 1 (`H-52`): **Jugar seleccionada** (w=150,x=-154) → `play(id)` tal cual está hoy; **Nueva instancia** (w=150,x=4) → `nav.create()`. Fila 2 (`H-28`), 4 botones de 72 con huecos de 4 (`x=-150,-74,2,78`): **Editar** (`configure(id)`), **Mods** (`nav.go('mods')`), **Explorar** (`nav.go('explore')`), **Atrás**.
- Deshabilitados: "Jugar" si `!selected || anyBusy && !sessions.isBusy(selected.id)`; "Editar/Mods" si no hay selección.
- Estados: vacío → `empty="Aún no tienes instancias. Crea la primera con «Nueva instancia»."` y solo "Nueva instancia" habilitado; aviso `anyBusy` → `McText` gris sobre los botones ("Hay una instancia en marcha. Cuando termine podrás iniciar otra.").
- `onactivate` (doble clic/Enter) = `play`.

### 6.3 Centro de sesión (`home`) — mezcla de plantilla A y B
- Datos: `instances.selected`, `sessions.get(id)` (fase, `progress`, `message`, `kind`, `startedAt`), `sessions.logOf(id)`.
- Parte superior (y=12..64): nombre de la instancia (blanco), línea de estado (texto actual de `statusText`), hora de inicio gris, `McProgress` (y=52) cuando hay `progress` o `phase` de preparación.
- Consola: `McList` con `rowHeight=10` y filas de una línea (texto `LogLine.text`, rojo `#ff5555` si `stream==='stderr'`, gris si es nivel DEBUG según el filtro actual); filtro con `McField` (placeholder "Buscar en la consola...") y `McCycle` "Nivel: Todo / Info / Avisos / Errores" → mismos `filter` y `level` que `Home.svelte`. Autoscroll al final salvo que el usuario haya subido (usar el mismo criterio actual); límite 1000 líneas lo hace el store.
- Botones: **Jugar/Cerrar juego** (primario; `sessions.launch`/`sessions.stop`), **Mods**, **Configurar**, **Reparar** (`sessions.repair`), **Cambiar instancia** (`nav.go('instances')`), **Limpiar** (`sessions.clearLog`, deshabilitado si no hay líneas). Distribución: fila `H-52` = Jugar/Cerrar (w=150) + Limpiar (w=150); fila `H-28` = Mods, Configurar, Reparar, Cambiar (4×72).
- Estado sin instancias (`instances.list.length===0`): mensaje "Crea tu primera instancia" + botón **Crear instancia** (`nav.create()`).
- Errores de sesión (`message`/`kind`): `McText` rojo bajo el título (máx. 2 líneas); detalle completo en `McDialog` si el usuario pulsa **Ver detalle**.

### 6.4 Crear instancia (`create`) y 6.5 Editar instancia (`edit`) — plantilla B + subpantallas
Mantén EXACTAMENTE las variables, validaciones, `busy`/`saving` y llamadas de `CreateInstance.svelte`/`EditInstance.svelte`. Reparte los controles así (nuevo estado local `sub: 'main' | 'version' | 'advanced' | 'icon' | 'more'`; "Atrás" de una subpantalla vuelve a `main`, no a `nav`):

**Crear – `main`:** `McTabs` Oficiales/Personalizado (y=28) → `source`; **Nombre** `McField` (maxlength 64); **Tipo** `McCycle` (`loader`, solo en Oficiales); botón **Versión: {version||'Elegir...'}** → `sub='version'` (en Oficiales) o **Versión instalada: ...** (Personalizado); botón **Icono: {nombre}** → `sub='icon'`; **Rendimiento** `McCycle` + `McText` del hint; **Opciones avanzadas...** → `sub='advanced'`; **Importar modpack...** (`importPack`, deshabilitado si `importing||busy`); abajo: **Crear instancia** (`type=submit`, mismas condiciones de `disabled`) / **Cancelar** (`nav.go('instances')`). Si `importing` muestra `McProgress` (`instances.importPercent`) en lugar del botón importar.
**Crear – `version` (Oficiales):** `McList` de `versions.list` filtrada por los toggles; fila: id (línea 1) y tipo+fecha (línea 2, `releaseTime` formateada); `McCycle` "Mostrar: Versiones / + Snapshots / + Beta / + Alpha" (traduce los 4 checkboxes `show[kind]` de `VersionPicker`: preserva su lógica de filtrado y `autoSelect`); estados `versions.loading` ("Buscando versiones de Minecraft..."), `versions.error` (rojo + botón **Reintentar** → `versions.load(loader)`), lista vacía (texto + **Buscar de nuevo**), y avisos de `versions.source` ('cached' → "Sin conexión: se muestra la última lista descargada.", 'installed' → "Sin conexión: solo aparecen las versiones que ya tienes instaladas."). Botón **Hecho** fija `version` y vuelve.
**Crear – `Personalizado`:** réplica de `CustomVersionPicker` (lee su archivo entero): lista de versiones instaladas fuera del launcher (`McList`), carpeta (`dir`) mostrada como `McText` + botón **Elegir carpeta...** (diálogo nativo vía el mismo comando que usa hoy), `McToggle` "Copiar datos".
**`icon`:** cuadrícula de iconos (3 filas × hasta 5; cada celda 36×36 con icono 32×32, seleccionada con marco blanco), opción 0 = "Por defecto" (valor `''`); `ITEM_ICONS` y `icon` igual que `IconPicker` (`value===''` ⇒ el del tipo). Teclado: flechas mueven la celda; `role="radiogroup"` con `role="radio" aria-checked aria-label={icon.label}`.
**`advanced`:** **Memoria** `McCycle` (Automática/Elegir cantidad) y, si es fija, `McSlider` GB 1–16 → `maxGb` (mantén la validación/formato de `maxGb`: el campo era texto; ahora entero 1–16); **Java** `McCycle` (Automático/Mi propio Java) y, si es ruta, `McField` "Carpeta de Java" con hint «La carpeta que contiene "bin"» y placeholder `C:\Program Files\Java\jdk-21`. Hecho/Atrás.
**Editar – `main`:** Nombre, botón Versión (`autoSelect={false}`), Icono, **Perfil que juega** (`McCycle` si ≤6, si no `McList`), Rendimiento, Opciones avanzadas..., **Guardar** (submit) / **Volver a Mis instancias**; botón **Más acciones...** → `sub='more'`: **Duplicar** (`duplicate`), **Exportar como modpack...** (`exportPack`), **Eliminar...** → `McDialog` de confirmación (`confirmDelete`) con el nombre; los mensajes de éxito/error de estas acciones aparecen como `McText` en la subpantalla.
- Errores de guardado: `McText` rojo (máx. 2 líneas) justo encima de la fila de botones. Mientras `saving`/`busy`, todos los controles `disabled`.

### 6.6 Explorar (`explore`) — plantilla A simplificada (3 filas grandes, sin lista scroll)
- **Decisión fija:** `McList` con 3 filas de `rowHeight=36` (sin scroll visible): icono 32×32 (grass-block; diamond-pickaxe con loader fabric; crafting-table), título blanco («Sin mods · La esencia de Minecraft», «Con mods · Un mundo a tu medida», «Modpack · Importa un modpack») y una línea gris de descripción acortada con `fitText` (la descripción completa va en `McText` bajo la lista, w=300, ≤3 líneas, para la fila seleccionada).
- Botones: **Crear instancia** (acción de la fila: `nav.create('vanilla')`, `nav.create('fabric')` o `importPack`, con `importing` deshabilitado y `McProgress` + error en rojo), **Atrás**. `onactivate` ejecuta la acción.

### 6.7 Herramientas (`tools`) — plantilla A
- `McList` con una fila por herramienta (icono, título, descripción): **Reparar instancia**, **Exportar modpack**, **Importar modpack** (+ tarjetas «Próximamente» solo si `FEATURES.comingSoonTools`, filas deshabilitadas con línea 3 "PRÓXIMAMENTE").
- Cabecera: "Instancia activa: {nombre}" (o "No hay ninguna instancia activa").
- Botón principal **Ejecutar** actúa sobre la fila: `repair` (`sessions.repair` + `nav.go('home')`), `exportPack`, `importPack`; deshabilitado por las mismas condiciones actuales (`!instance||busy`, `working`). `notice` en verde y `error` en rojo con `McText` (2 líneas) entre lista y botones; `McProgress` para `instances.importPercent`. **Atrás**.

### 6.8 Perfiles (`accounts`) — plantilla A + subpantalla
- Lista de `accounts.list`: icono (cara de la skin del perfil si existe, si no el bloque de hierba) + `label` + línea 2 ("Sin conexión" / "Microsoft") + línea 3 estado (`accounts.states[id]`, p. ej. "Sesión caducada") ; marca "Perfil activo" en verde para `accounts.defaultId`.
- Botones: **Usar** (`accounts.setDefault`), **Quitar...** (`McDialog` → `accounts.remove`), **Volver a iniciar sesión** (solo Microsoft; `connect`), **Añadir perfil...** → subpantalla `new`, **Skins** → `nav.go('skins')`, **Atrás**.
- Microsoft: solo visible si `FEATURES.microsoftSettings` o ya existe una cuenta Microsoft. **Conectar cuenta** (`connect`) deshabilitado si `!accounts.microsoft.enabled`; mientras `loginInProgress`, `McText` con `accounts.loginMessage` + botón **Cancelar** (`accounts.cancelLogin`).
- Subpantalla `new` y pantalla `Welcome`: título, `McField` "Nombre" (maxlength 16, placeholder "Alex", `error` visible), **Crear perfil** (submit, `accounts.createLocal(name)`), **Cancelar**. Welcome no tiene Cancelar ni Atrás; el título es "Bienvenido".

### 6.9 Skins (`skins`) — plantilla A con panel de vista previa
- Izquierda: `McList` de `skins.list` (icono = `skins.previews[id]` recortado a la cara 32×32 con `image-rendering: pixelated`; nombre; "Clásico/Fino"; "En uso" verde si es la del perfil). Derecha (si W≥400) o encima (si no): `McViewport` con `SkinPreview3D` de la skin seleccionada (reemplaza `SkinPreview`).
- Botones: **Usar** (`use`), **Editar** (`openEditor(id)`), **Importar skin...** (`importSkin`), **Crear skin nueva** (`openEditor(null)`), **Más...** → subpantalla (Renombrar → `McField` + Guardar nombre/Cancelar; Exportar; Eliminar... → `McDialog`), **Quitar skin del perfil** (`unassign`), **Atrás**. Mismas condiciones `disabled` (`busy`, `!profile`, `!profileSkinId`).
- Vacío: "Aún no tienes skins" + Importar/Crear habilitados.

### 6.10 Editor de skins (`skinEditor`) — layout propio, el más delicado
Conserva íntegras la lógica de `routes/SkinEditor.svelte` y `lib/skinEditor.ts`/`skinGeometry.ts` (pintura, deshacer/rehacer, capas, espejo, cuadrícula, zoom, guardado y exportación). Cambia solo la presentación:
- **Lienzo**: `McViewport` 64×64 texels con `zoom` entero (el slider 4–14 original se sustituye por `zoom = clamp(floor((H-84)/64), 1, 3)` GUI-px por texel → a 240 de alto: 2 (128 px); a 280: 3 (192 px)). Ratón: coordenadas `floor((clientX - rect.left) / (zoom·s))`.
- **Columna derecha** (x desde el lienzo + 8): `McViewport` 3D (`SkinPreview3D`) de 80×100; debajo herramientas como 4–6 botones de 24×20 con icono bitmap (lápiz, borrador, cubo, cuentagotas… según `tools`; tooltip = `aria-label` + atajo `key`), `aria-pressed`.
- **Color**: no existe `<input type=color>` en este estilo → paleta propia: cuadrícula 8×4 de swatches 10×10 (16 colores base de Minecraft + 16 de la skin actual extraídos) + 3 `McSlider` R/G/B (0–255) + muestra del color actual con `#RRGGBB` bitmap. `color` sigue siendo un string hex como hoy.
- **Opciones** (subpantalla `options` con `McToggle`/`McCycle`): Capa (Base/Exterior), Mostrar capa base/exterior, Espejo, Cuadrícula, Grupo del cuerpo (`group`, `McCycle`), Modelo (Clásico/Fino), Nombre (`McField`).
- **Botones** (abajo): **Deshacer**, **Rehacer**, **Opciones...**, **Guardar** (submit), **Guardar y exportar PNG...**, **Volver** (si hay cambios sin guardar → `McDialog` "¿Descartar cambios?", comportamiento actual de `back`).
- Atajos de teclado actuales se conservan (Ctrl+Z/Y, teclas de herramienta).

### 6.11 Ajustes (`settings`) — plantilla B con subpantallas
- Principal: **Apariencia: {Sistema/Claro/Oscuro}** (`McCycle` → `theme`), **Tema especial...** → sub `themes`, **Velocidad de descarga: {…}** (`McCycle` → `speed`, con hint de la opción actual), **Carpeta de datos** (solo lectura: `settings.dataDir` en `McText` gris con "...", tooltip = valor completo en `aria-label`), campo Java ("Java: carpeta propia", `settings.value.javaPath`, hint "Solo se usa si es compatible con la versión de Minecraft; si no, el launcher elige uno adecuado."), y si `FEATURES.microsoftSettings`: `McField` "ID de aplicación de Microsoft" con su hint. **Guardar** (submit → `settings.save`, `saving` → "Guardando...") / **Atrás**.
- Sub `themes`: `McList` con `settings.themes` (nombre, autor/descripción), el seleccionado se aplica con **Usar tema**; **Importar tema...** (`importTheme`), **Quitar este tema** (`removeTheme`, deshabilitado si `!themeRemovable`, con `McDialog`). Ver §7 sobre el efecto de los temas.

### 6.12 Mods (`mods`) — plantilla A con `McTabs`
Conserva `routes/Mods.svelte` y `stores/mods.svelte.ts` (lee ambos completos antes de empezar: ~420 líneas de lógica).
- Si la instancia no admite mods (caso "La gestión de mods no está disponible aquí"): mensaje `McText` + **Crear instancia con mods** (`nav.create('fabric')`) + **Volver**.
- Tab **Mis mods**: `McList` (icono por loader, nombre, versión, estado), `McToggle` por fila → en lista no caben toggles: usa botón **Activar/Desactivar** (`mod.enabled`) para la fila seleccionada; botones: **Agregar a...** (`mods.addLocal`), **Optimizar** (`mods.optimize`), **Buscar actualizaciones** (`mods.checkUpdates`), **Actualizar** (aparece si la fila tiene `update`), **Revertir** (`mods.rollback`), **Quitar...** (`McDialog`), **Volver**. Todo deshabilitado según `running || mods.busy !== null`, igual que hoy. Si `running`, `McText` amarillo: "Cierra el juego para cambiar mods."
- Tab **Buscar mods**: `McField` "Buscar" (hint «Solo se muestran mods compatibles con tu versión.», placeholder «Sodium, Lithium, minimapa…») + **Buscar** (`mods.busy==='search'` → "Buscando..."); resultados en `McList` (icono remoto: **no cargar imágenes remotas** por CSP → usa icono genérico; nombre, autor, descripción corta); **Instalar** (`mods.install`). 
- `mods.busy` distinto de null → `McProgress` indeterminado con el nombre de la acción.

### 6.13 Ayuda
`McDialog` abierto desde el botón de idioma del menú principal. Contenido: el texto real de `HelpDialog.svelte` (no el de la maqueta), partido con `McText` (w=300), botón **Hecho**. Si el texto excede `H-80`, `McScroll`.

### 6.14 Estados transversales (todas las pantallas)
Cada pantalla que cargue datos debe tener y probarse en cuatro estados: **cargando** (texto gris centrado "Cargando..."), **vacío**, **error** (rojo + acción de reintento si existe), **ocupado** (controles `disabled` + progreso). Bloquear doble envío igual que hoy (`busy`/`saving`/`importing`).

## 7. Stores, temas y cambios permitidos fuera del template

- `nav.svelte.ts`: solo añadir la vista `'menu'` (§6.1). Test en `vitest` de `viewLabel`/`navParent`.
- `instances.svelte.ts`, `sessions.svelte.ts`: no se cambian salvo exponer, si falta, `selectedId` (ya existe). Ningún cambio de firma.
- `theme.ts`/temas JSON: los tokens CSS (`--bg`, `--action`…) ya no gobiernan el aspecto (los sprites son PNG fijos). Decisión fija **A**: los temas solo cambian (1) `--shade` (tinte del fondo, `rgba(var(--shade), .75)`) derivado de `--bg`, y (2) el color del texto resaltado (hover) derivado de `--action` cuando tenga contraste ≥ 4.5 con el fondo. El tema azul incluido → shade azulado. `typography.scale × density.uiScale` (zoom del documento) **se desactiva** para el stage (rompería el pixel-perfect): el zoom no se aplica en modo Minecraft; actualiza `theme.test.ts` en consecuencia.
- `src-tauri/capabilities`: solo `core:window:allow-close` si falta (§6.1).

## 8. Limpieza (solo cuando todo esté migrado)

- Elimina de `app.css` lo que nadie use (comprueba con búsqueda). Deja reset mínimo, `.sr-only`, `:focus-visible`, y mueve el resto a `mc/stage.css`.
- Retira `public/fonts` y `@font-face` (Space Grotesk, IBM Plex) y los componentes sin referencias (`Card`, `Avatar`, `Icon`, `PageHeading`, `Select`, `Input`, `ProgressBar`, `ConfirmDialog`, `HelpDialog`, `IconPicker`, `VersionPicker` si su lógica ya vive en las nuevas pantallas — **antes de borrar `VersionPicker`/`CustomVersionPicker`/`PerformanceOptions`/`IconPicker` extrae su lógica a funciones/stores reutilizables y cúbrela con tests**).
- Los SVG de `Icon.svelte` no se portan.

## 9. Verificación (sin incertidumbres)

### 9.1 Comandos (en `apps/desktop`)
```bash
npm run check   # svelte-check + tsc: 0 errores
npm test        # vitest: todo en verde (incluye los nuevos tests de mc/logic.ts, font.ts, nav)
npm run build   # build de producción
npm run dev     # revisión visual con el backend simulado (lib/api/mock.ts)
npm run tauri dev   # al menos una vez con backend real (§9.5)
```
Además: `grep -rn "https\?://" src --include=*.svelte --include=*.ts --include=*.css` no debe mostrar recursos remotos (los enlaces del `mock.ts` de Modrinth están permitidos solo si ya existían), y la consola del navegador no debe mostrar violaciones de CSP.

### 9.2 Auditor automático de overflow (créalo: `src/mc/audit.ts`, expuesto solo en `import.meta.env.DEV` como `window.__mcAudit()`)
Para el DOM actual devuelve una lista de problemas; cada problema = `{screen, kind, element, rect, container}`. Reglas:
1. **Texto fuera de su control:** para cada `canvas.bitmap-text` dentro de `.mc-button, .mc-field, .mc-slider, .mc-row, .mc-list, .mc-dialog`, su `getBoundingClientRect()` debe quedar dentro (tolerancia 0) del rect del contenedor más cercano (sin contar el 1 px de sombra a la derecha/abajo, que se descuenta explícitamente).
2. **Fuera de pantalla:** todo control y texto debe estar dentro de `[0,W]×[0,H]` del stage.
3. **Solapes:** no puede haber dos controles interactivos con rects que se intersecten (excepto el padre y su hijo directo), ni dos `canvas` de texto que se intersecten con área > 0.
4. **Enteros:** `left/top/width/height` de cada elemento del stage en px GUI deben ser enteros (`Number.isInteger`).
5. **Texto HTML visible:** ningún nodo de texto con `trim() !== ''` visible fuera de `.sr-only`/`<input>`/`<title>`.
6. **Foco:** cada elemento interactivo es alcanzable con Tab y muestra marco de foco.
7. **Tamaño táctil/click:** cada control interactivo mide ≥ 40 px físicos (`rect.width/height ≥ 40`).

### 9.3 Matriz de pruebas (todas las pantallas × tamaños × datos)
Ejecuta `__mcAudit()` en cada celda; **0 problemas** exigidos.

- **Tamaños de ventana** (redimensionar con el navegador o `resize_window`): 820×560, 1100×720, 1920×1080, y una prueba forzada a **GUI 320×240** (fija `window.__forceGui = {w:320,h:240}`, que el stage respete en DEV).
- **Pantallas:** menú, instances, home (sin instancia / con instancia / jugando / error), create (cada `sub`, ambas pestañas), edit (cada `sub`), explore, tools, accounts (+new), welcome, skins (+more), skinEditor (+options), settings (+themes), mods (sin soporte / mis mods / buscar), ayuda y cada `McDialog`.
- **Datos extremos (con el mock `lib/api/mock.ts`, ampliándolo en DEV si hace falta):**
  - nombre de instancia de 64 caracteres, y con `ñ ü ¿ ¡ « » é`;
  - versión larga (`1.21.4-forge-54.0.12`) y loader larguísimo;
  - perfil de 16 caracteres; skin con nombre de 48;
  - 0, 1, 7 y 500 instancias/skins/mods; 3 000 versiones;
  - error de 300 caracteres; mensajes de varias líneas (`\n`);
  - emoji o carácter sin glifo (debe mostrarse `?`, ancho coherente, sin romper);
  - texto de log de 2 000 caracteres (se acorta a la derecha con "..."; el filtro sigue buscando en el texto completo);
  - sin conexión (`navigator.onLine=false`, `versions.source` = 'cached'/'installed').
- **Entrada:** teclado completo (Tab/Shift+Tab, Enter, Espacio, flechas, Esc, RePág/AvPág/Inicio/Fin en listas), ratón (hover, clic, doble clic, rueda, arrastre de barra y de slider), y `prefers-reduced-motion`.

### 9.4 Pruebas funcionales (paridad con la app anterior)
Recorre `docs/QA_MATRIX.md` y marca cada fila como "igual que antes". Mínimo, con mock y luego con backend real:
1. Primer arranque → Welcome → crear perfil offline (nombre vacío y de 17 caracteres deben fallar con el mismo mensaje).
2. Crear instancia vanilla → aparece en Mis instancias seleccionada → Jugar → progreso con fases reales → consola con líneas reales → filtro/nivel/Limpiar → Cerrar juego.
3. Crear con Fabric → Mods → buscar → instalar → desactivar → actualizar → revertir → quitar.
4. Editar: cambiar nombre/versión/icono/memoria/Java/perfil → Guardar → persiste; Duplicar; Exportar modpack; Eliminar (con confirmación, y Cancelar no elimina).
5. Importar `.mrpack` desde Explorar y desde Herramientas (progreso, error con archivo inválido).
6. Perfiles: usar, quitar (confirmación), Microsoft deshabilitado/oculto según `FEATURES`.
7. Skins: importar PNG, asignar al perfil, crear en el editor, pintar, deshacer/rehacer, guardar, exportar, renombrar, eliminar.
8. Ajustes: cambiar apariencia, velocidad, tema JSON (importar/quitar), Java; reabrir y comprobar que persiste.
9. Salir cierra la ventana (solo en Tauri).

### 9.5 Verificación en Tauri real
`npm run tauri dev`: el panorama y la fuente cargan bajo la CSP real; redimensionar la ventana de 820×560 a pantalla completa y con DPI 100/125/150 % (Windows) mantiene nitidez (sin bordes borrosos entre texturas; `image-rendering: pixelated`); la consola real recibe `launch://stdout|stderr`; CPU en reposo del panorama < ~5 % y 0 % con la ventana minimizada o `prefers-reduced-motion`.

### 9.6 Criterios de "terminado" (checklist)
- [ ] `check`, `test`, `build` en verde; tests nuevos para `font`, `wrap`, `logic` de lista/scroll, `nav`.
- [ ] `__mcAudit()` = 0 problemas en toda la matriz de §9.3.
- [ ] Cero texto HTML visible; cero recursos remotos; cero errores de CSP.
- [ ] Paridad funcional completa (§9.4) y `docs/QA_MATRIX.md` actualizada.
- [ ] Navegación solo por teclado completa; roles/labels correctos (probar con el Narrador de Windows al menos en menú, lista y diálogo).
- [ ] `docs/UI_MIGRATION_MINECRAFT.md` escrito, `UI_MIGRATION.md` marcado como reemplazado, README con créditos.
- [ ] Capturas antes/después de cada pantalla en `docs/reference/minecraft-ui/` (opcional pero recomendado).

## 10. Conflictos con las reglas actuales del destino

- `UI_MIGRATION.md` reglas 2–3 (clases globales de `app.css`, tokens de tema): obsoletas → documentadas en el nuevo archivo (§7 decisión A).
- Regla 7 (objetivos ≥ 40 px): cumplida por el factor de escala (≥ 2× a 820×560); auditada por la regla 7 de §9.2.
- Spec 08 (design system moderno): añadir sección "Modo Minecraft" y marcar la anterior como histórica.
- Iconos de instancia: el destino ya reescala los PNG a 64 px; en lista se dibujan a 32×32 GUI px sin suavizado.
- Skins de la maqueta (colores por parte): no se portan; se usan las skins reales del store.
- Textos: mantener «personas no técnicas» (nada de términos internos fuera de Opciones avanzadas).

## 11. Recomendaciones para el agente

- **Orden de trabajo:** §3 infraestructura → §5 componentes (con tests) → pantallas en este orden: 6.1 menú, 6.2 instancias, 6.3 sesión, 6.4/6.5 crear/editar, 6.6/6.7 explorar/herramientas, 6.8 perfiles/bienvenida, 6.11 ajustes, 6.9 skins, 6.12 mods, 6.10 editor de skins (el último). Tras cada pantalla: auditor a 0, `check`, `test`, commit propio (`ui(mc): <pantalla>`).
- No avances a la 6.4 hasta que 6.1–6.3 pasen la matriz de §9.3: prueban stage, fuente, botones, lista, progreso y consola.
- Reutiliza coordenadas de la maqueta (`src/App.svelte` de la maqueta: busca el bloque de cada `screen`); si una pantalla necesita más sitio, **divide en subpantalla**; nunca reduzcas fuente, nunca `overflow: visible`, nunca recortes a ojo.
- Textos de botón en español, ≤ ~22 caracteres a w=200 (≤ 10 a w=72). Lo que no quepa se acorta con "..." y el texto completo va en `aria-label`.
- No mezcles refactor de lógica con estética; si detectas un bug de lógica, anótalo en `docs/PROGRESS.md` en vez de arreglarlo dentro del commit visual.
- Si falta un dato que la UI necesita y no existe en el contrato (`docs/IPC.md`), documéntalo como pendiente; no lo inventes.
- Prueba de humo final = §9.4 completo con backend real.
