# 09 — Themes JSON

## Objetivo

Preparar desde el primer CSS un sistema tokenizado.

MVP:
- default dark;
- default light.

Futuro:
- themes JSON de usuario.

## Archivo

```json
{
  "id": "disorderlaucher.default.dark",
  "name": "Default Dark",
  "schemaVersion": 1,
  "colors": {},
  "typography": {},
  "shape": {},
  "density": {},
  "effects": {}
}
```

## Seguridad

Tema = datos.

No permitir:
- JS;
- HTML;
- CSS arbitrario;
- URLs ejecutables;
- imports remotos.

## Theme engine

Rust valida.

Frontend aplica CSS variables.

## Integración temprana

Aunque themes custom lleguen después:
- todos los colores deben venir de tokens;
- radios desde tokens;
- tipografía desde tokens;
- no hardcodear componentes con valores no tematizables.

## Compatibilidad

`schemaVersion`.

Fallback al tema default si falla.

## Directorio

```text
themes/
  default-dark/
  default-light/
  custom/
```

## Descarga futura

Validar:
- schema;
- tamaño;
- hash;
- compatibilidad.

No ejecutar paquetes.
