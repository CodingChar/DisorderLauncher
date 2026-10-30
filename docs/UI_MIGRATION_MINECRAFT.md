# Interfaz estilo Minecraft 1.7.10 (maqueta → `apps/desktop`)

> **Si eres otro chat/agente: lee este archivo antes de tocar `apps/desktop/src`.**
> Esta migración **reemplaza** la reforma anterior. Origen visual: la maqueta `maqueta/`
> (menús de Minecraft 1.7.10). Estado: **migrada** (2026-10-18).

## Qué cambió y qué NO

- Cambió **solo la capa visual**: todo `src/routes/*.svelte` conserva sus `$state/$derived`, llamadas a `api.*`/stores, validaciones y manejo de errores; solo se reescribió su *template*.
- **Intactos**: `src/lib/api`, `src/lib/stores/*` (salvo `nav.svelte.ts`: se añadió la vista `'menu'`), `src-tauri/` (salvo el permiso `core:window:allow-close` en `capabilities/default.json`) y `crates/`.
- La lógica que vivía dentro de componentes visuales se extrajo a módulos con tests antes de borrarlos:

| Antes | Ahora |
|---|---|
| `VersionPicker.svelte` | `lib/versionPicker.ts` (puro, probado) + `lib/versionPicker.svelte.ts` (`useVersionPicker`) |
| `CustomVersionPicker.svelte` | `lib/customVersions.svelte.ts` (`CustomVersions`) |
| `PerformanceOptions.svelte`, validaciones de memoria/Java | `lib/instanceForm.ts` (puro, probado) |
| `IconPicker.svelte` | `routes/parts/IconScreen.svelte` + `lib/mc/McIconGrid.svelte` |
| `ConfirmDialog`, `HelpDialog` | `lib/mc/McDialog.svelte` (+ `lib/helpText.ts`) |
| `ProgressBar` | `lib/mc/McProgress.svelte` |
| `SkinPreview3D`, `SkinPreview` | `lib/mc/SkinPreview3D.svelte` (misma geometría); la cara 2D de las listas es un recorte de la textura |

## Reglas (vigentes para cualquier cambio de interfaz)

1. **Escenario en píxeles de GUI.** `McStage` calcula `factor = max(1, floor(min(W·dpr/320, H·dpr/240)))` y expone `--s` (= factor/dpr), `--W`, `--H`, `--cx`. Todo se coloca con coordenadas **enteras** y posición absoluta: `x` desde el centro horizontal, `y` desde arriba (o desde abajo con `anchor="bottom"` e `y` negativa). Nada de flex/grid fluido dentro del escenario. Toda pantalla debe funcionar para cualquier `W ≥ 320` y `H ≥ 240`.
2. **Todo texto visible es un canvas con la fuente bitmap** (`BitmapText`, `ascii.png`). Nada de texto HTML visible: solo `.sr-only`, `aria-label` e `<input>` invisibles. Todo texto lleva ancho máximo: `fitText` (acorta con «...») o `wrap`/`McText` (parte por palabras, corta palabras gigantes, respeta `\n`, `maxLines`).
3. **La fuente no tiene todos los signos**: `…` se dibuja como `...`, `·` y las rayas como `-`, las comillas curvas como rectas, y una vocal acentuada que falte (Á, Ï…) usa la letra base; lo demás sale `?`. Ver `normalizeText` en `lib/mc/font.ts`.
4. **Una pantalla = como mucho ~7 controles en columna a 240 de alto.** Si hay más, subpantalla (`sub`) o columnas de 150 px; nunca se reduce la fuente ni se recorta a ojo.
5. **Sin recursos remotos** (CSP `default-src 'self'`): assets en `public/assets/`, rutas `/assets/...`. Ya no se incluyen fuentes web.
6. **«Atrás» y Esc** comparten una pila (`lib/mc/back.ts`): un diálogo o una subpantalla registran su salida con `pushBack`; si no hay ninguna, `goBack()` usa la tabla `navBack`. Edición y Mods vuelven a donde se abrieron.
7. Texto para personas no técnicas (regla de `CLAUDE.md`); lo que no existe se muestra como «Próximamente» solo si `FEATURES` lo permite.
8. Tras cualquier cambio: `npm run check`, `npm test`, `npm run build` y `window.__mcMatrix({ seed: true })` en `npm run dev` (ver «Verificación»).

## Componentes (`src/lib/mc/`)

| Componente | Notas |
|---|---|
| `McStage` | Fondo (panorama 3D giratorio en el menú, tierra tintada en el resto; el giro es una animación CSS de la GPU con la fórmula del menú original —yaw 2°/s, pitch `sin(t)*25+20`— que no depende de la preferencia de animaciones de Windows y se apaga en Ajustes › «Fondo del menú», ver `lib/mc/motion.ts`), escala entera, contexto `useGui()` con `W/H/cx/s/factor`. En DEV `window.__forceGui = {w, h}` fuerza el tamaño virtual. |
| `McScreen` | Título bitmap centrado + `<h1 class="sr-only">` que recibe el foco al abrir la pantalla. |
| `BitmapText` · `McLabel` · `McText` | Texto suelto, etiqueta posicionada y párrafo. `McText` con `role="alert"/"status"` para errores y avisos. |
| `McButton` | Sprite de 200×20, hover amarillo (del tema), deshabilitado, `active` (pestaña/herramienta), `type="submit"`, `extra` para roles y eventos. |
| `McField` | `<input>` invisible + texto bitmap; `type="password"`, `error`, `hint`, `onenter`, `autofocus`; el final del texto siempre se ve (`tailText`). |
| `McSlider` · `McCycle` · `McToggle` · `McTabs` | Ajuste numérico, selector circular (clic/→ siguiente, clic derecho/← anterior), interruptor (`role="switch"`) y pestañas (`role="tablist"`). |
| `McList` | Lista tipo `GuiSlot`: filas de 36 (o 24/10), virtualizada (solo las visibles ±2), rueda de 18 px, barra arrastrable, teclado completo, `role="listbox"`, estados cargando/vacía. Lógica pura en `logic.ts`. |
| `McProgress` | Determinada o indeterminada (estática con `prefers-reduced-motion`). |
| `McDialog` | Modal con panel opaco, `inert` en lo de alrededor, trampa de Tab, foco en el botón de cancelar, `scroll` para textos largos. |
| `McViewport` · `SkinPreview3D` · `McIconGrid` | Marco para contenido que no es texto, vista 3D de skin y rejilla de iconos (`role="radiogroup"`). |

## Mapa de pantallas

| Vista (`nav.view`) | Archivo | Notas |
|---|---|---|
| `menu` (nueva, inicial) | `routes/MainMenu.svelte` | Panorama, logo, splash, pie, botón de ayuda, **Salir** (`getCurrentWindow().close()`, deshabilitado fuera de Tauri). |
| `instances` | `Instances.svelte` | Lista + Jugar/Nueva/Editar/Mods/Explorar/Atrás. |
| `home` | `Home.svelte` | Centro de sesión: progreso, consola real (hasta 520 px de ancho, las líneas largas continúan en la fila siguiente, botón Copiar porque el texto bitmap no se selecciona; lista con seguimiento del final), diálogo de fallo. |
| `create` · `edit` | `CreateInstance.svelte`, `EditInstance.svelte` | Subpantallas: versión, versión instalada (Personalizado), icono, opciones avanzadas, más acciones, perfil. Partes compartidas en `routes/parts/`. |
| `explore` · `tools` | `Explore.svelte`, `Tools.svelte` | Listas con acción. |
| `accounts` (+ nuevo perfil) · bienvenida | `AccountPicker.svelte`, `parts/ProfileForm.svelte`, `Welcome.svelte` | Microsoft solo si `FEATURES`/ya hay cuenta. |
| `settings` | `Settings.svelte` | Subpantallas: tema especial (con confirmación al quitar) y avanzado (ID de Microsoft, solo con `FEATURES.microsoftSettings`). |
| `skins` · `skinEditor` | `Skins.svelte`, `SkinEditor.svelte` | Editor: lienzo con zoom entero 1–3, paleta de 32 colores + R/G/B, opciones en subpantalla. |
| `mods` | `Mods.svelte` | Pestañas «Mis mods» / «Buscar mods». La búsqueda tiene orden, panel «Filtros» (categorías de casillas, entorno, compatibilidad, fuente si hay más de un proveedor), total de resultados, detalle del mod elegido y «Ver más» (paginación). Todo viene del contrato neutro de `mods_search`/`mods_catalog`: los textos están en `lib/modCatalog.ts` y un id sin nombre se muestra tal cual. |

## Temas

Los sprites son PNG fijos, así que un tema solo cambia (1) `--shade` (tinte del fondo, derivado de su color de fondo; los fondos claros se oscurecen) y (2) el color de resalte del texto (de `colors.action` si contrasta ≥ 4.5 con el fondo; si no, `#ffffa0`). `typography.scale × density.uiScale` **ya no se aplica** (rompería la rejilla de píxeles). Lógica en `lib/mc/color.ts`.

## Verificación

```bash
cd apps/desktop
npm run check   # svelte-check + tsc
npm test        # vitest (incluye font, wrap, listas/scroll, navegación, colores, formularios, versiones, editor)
npm run build
npm run dev     # navegador con el backend simulado (lib/api/mock.ts)
```

En el navegador (solo DEV):

- `window.__mcAudit()` audita la pantalla actual: texto fuera de su control, fuera de pantalla, controles y textos solapados, medidas no enteras, texto HTML visible, foco y tamaño de objetivo ≥ 40 px físicos. Los controles bajo un diálogo (`inert`) no cuentan. Las muestras de la paleta del editor llevan `data-audit-small` (son un selector compacto, con alternativa de teclado y los deslizadores R/G/B).
- `await window.__mcMatrix({ seed: true, extra: 500 })` recorre **todas** las pantallas, subpantallas y diálogos con datos extremos (nombres con `ñ ¿ ¡ « »`, de 64 caracteres, con emoji o salto de línea, 500+ instancias) y devuelve la lista de problemas (vacía = correcto). Se repite a 820×560, 1100×720, 1920×1080 y forzando `window.__forceGui = { w: 320, h: 240 }`.
- Ver `docs/QA_MATRIX.md` para lo verificado y lo pendiente.

## Pendiente conocido

- Los iconos de resultados de Modrinth no se cargan (CSP): se usa un icono genérico.
- El editor de skins ya no dibuja el nombre de cada parte sobre el lienzo (solo contornos de color); el nombre de la parte se elige en «Opciones...».
- El campo de texto no dibuja la selección parcial del ratón (el `<input>` real sí la gestiona); solo «seleccionar todo» se resalta.
- «Perfiles y skins» son dos pantallas enlazadas (Perfiles → Skins), no una.
- `docs/spec/08_VISUAL_DESIGN_SYSTEM.md` conserva el sistema anterior como histórico; ver su nota «Modo Minecraft».

## Créditos

Texturas y fuente inspiradas en Minecraft, © Mojang. DisorderLauncher no está afiliado a Mojang ni a Microsoft.

## Sonido de interfaz

- Modulo unico: `apps/desktop/src/lib/audio/sfx.ts` (Web Audio, buffers decodificados al arrancar via `preloadCore`). Uso: `playSfx('click' | 'back')`.
- Efectos en `apps/desktop/public/assets/sounds/`. Para anadir uno: copiar el mp3 y registrarlo en `SOUNDS`.
- `McButton` suena solo (`sound="auto"`): etiquetas Atras/Volver/Cancelar/Salir/Cerrar/Hecho/No -> `back`, el resto -> `click`. `sound="none"` lo silencia. `McList` (fila), `McIconGrid`, `McTabs` y `McToggle`/`McCycle` heredan el clic.
- Esc: `App.svelte` y `McDialog` reproducen `back`.
- Volumen: deslizador en Ajustes; se guarda en `localStorage` (`disorder.sfxVolume`).

## Textos acortados: nunca se pierden (2026-10-27)

Todo texto que no cabe se acorta con «...» **y** ofrece verlo entero:

- `McText` con `maxLines`, `McLabel` con `max` y las filas de `McList` muestran una marca amarilla **`[+]`** al final de lo acortado (`McMore`). Pulsarla abre una ventana (`McDetailHost`, un `McDialog` con scroll) con el texto completo. En una lista, `Espacio` hace lo mismo con la fila seleccionada.
- `McDialog` pasa solo a scroll cuando el mensaje no cabe, asi que sus mensajes nunca se cortan. Los dos sitios donde la marca no tiene sentido (el cuerpo del propio dialogo) la desactivan con `expandable={false}`.
- `McButton` pone el texto completo como `title` (tooltip) cuando lo acorta.
- En pantallas con espacio para dos avisos (Mods), lo que sobra se junta en la segunda linea con `[+]` en vez de descartarse.
- Para mostrar un detalle desde codigo: `detail.show(titulo, texto)` (`lib/mc/detail.svelte.ts`).
