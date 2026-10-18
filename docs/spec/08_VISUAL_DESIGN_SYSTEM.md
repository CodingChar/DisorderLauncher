# 08 — Diseño visual

## Objetivo

Interfaz profesional, monocromática, clara e intuitiva.

## Modo Minecraft (vigente desde 2026-10-18)

La interfaz de `apps/desktop` imita los menus de Minecraft 1.7.10: escenario en pixeles de GUI con escala entera, texto con la fuente bitmap
original, sprites de boton y listas tipo `GuiSlot`. Reglas, componentes, mapa de pantallas y verificacion en
[`../UI_MIGRATION_MINECRAFT.md`](../UI_MIGRATION_MINECRAFT.md). Implementacion: `apps/desktop/src/lib/mc/` y `src/mc/*.css`; sin fuentes ni recursos remotos.
Los temas JSON solo cambian el tinte del fondo y el color de resalte del texto (ver ese documento).

## Sistema anterior (historico)

Lo que sigue en este documento describe el diseno moderno anterior (maqueta `DisorderLauncherUI/`, `docs/UI_MIGRATION.md`, tokens de `app.css`):
ya no se aplica a la interfaz. Se conserva como referencia de intenciones (accesibilidad, claridad, lenguaje no tecnico), que siguen vigentes.
Referencia historica: `reference/disorderlaucher_reference.html`.

## Tipografía

Display:
- Space Grotesk 600/700.

Texto:
- IBM Plex Sans 400/500/600.

## Tema oscuro

```json
{
  "background": "#0C0C0C",
  "surface": "#141414",
  "surfaceAlt": "#1D1D1D",
  "surfaceRaised": "#252525",
  "textPrimary": "#F0F0EA",
  "textSecondary": "#A0A09A",
  "border": "#2D2D2D",
  "borderStrong": "#666666",
  "action": "#F3F3ED",
  "actionText": "#0C0C0C"
}
```

## Tema claro

```json
{
  "background": "#ECECE8",
  "surface": "#F8F8F5",
  "surfaceAlt": "#DFDFDB",
  "surfaceRaised": "#CFCFCA",
  "textPrimary": "#0A0A0A",
  "textSecondary": "#55554F",
  "border": "#C8C8C2",
  "borderStrong": "#80807A",
  "action": "#101010",
  "actionText": "#F4F4EF"
}
```

## Tema azul oscuro

Tema incluido `disorderlaucher.default.blue` (modo oscuro), accesible desde el boton de la luna en la cabecera y desde Ajustes:

```json
{ "background": "#0B1220", "surface": "#111D30", "surfaceAlt": "#192940", "surfaceRaised": "#243954",
  "textPrimary": "#E4EDF9", "textSecondary": "#9CADC6", "border": "#283C57", "borderStrong": "#526E92",
  "action": "#B8D4FA", "actionText": "#0B1729" }
```

## Estructura de la interfaz

Barra lateral (marca, navegacion, estado sin conexion, ajustes, ayuda, perfil activo, version) + cabecera (migas, estado, tema, perfil) + contenido con `page-heading` y pie.
Navegacion: Inicio, Mis instancias, Explorar, Herramientas, Perfiles, Skins; Ajustes y Ayuda abajo.

## Política de bordes

- 0 px: estructuras técnicas específicas.
- 7-8 px: inputs y botones.
- 11-13 px: tarjetas y paneles.
- 18 px: superficies principales.
- 50%: solo círculos funcionales.

Equilibrar recto/redondeado.

No redondear todo.

## Inicio

Debe priorizar:

```text
Instancia activa
Version | Loader | JUGAR

Estado básico
RAM | Mods | Java

Instancias recientes

Perfil
Offline Gaming / Microsoft

Preferencias
```

## Account picker

Diseñar desde el inicio para ambos providers:

```text
Perfiles

David
Offline Gaming
[Usar]

Microsoft
[Conectar cuenta]
```

En MVP, Microsoft puede estar feature-gated.

## Estado de red

Mostrar con lenguaje humano:
- “Puedes jugar sin conexión”.
- “Faltan archivos; conecta a internet”.
- “Tu cuenta Microsoft necesita volver a iniciar sesión”.

## Principio de simplicidad

No exponer:
- XSTS;
- PKCE;
- classpath;
- token;
- artifact.

Eso pertenece a diagnósticos avanzados.

## Accesibilidad

- AA;
- focus visible;
- targets >= 40 px;
- teclado;
- reduced motion.
