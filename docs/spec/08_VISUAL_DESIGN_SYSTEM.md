# 08 — Diseño visual

## Objetivo

Interfaz profesional, monocromática, clara e intuitiva.

Referencia:
`reference/disorderlaucher_reference.html`

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
  "borderStrong": "#4D4D4D",
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
  "textSecondary": "#62625E",
  "border": "#C8C8C2",
  "borderStrong": "#9E9E98",
  "action": "#101010",
  "actionText": "#F4F4EF"
}
```

## Política de bordes

- 0 px: estructuras técnicas específicas.
- 8 px: inputs.
- 10-12 px: botones.
- 16-18 px: tarjetas secundarias.
- 20-26 px: superficies principales.
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
