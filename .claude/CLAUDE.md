# DisorderLauncher — Agent Rules

## Lectura obligatoria

Lee TODOS los .md en `docs/spec/` antes de modificar codigo.

**Interfaz estilo Minecraft (vigente desde 2026-10-18)**: antes de tocar `apps/desktop/src` lee `docs/UI_MIGRATION_MINECRAFT.md` (reglas, componentes, mapa y verificacion).
`docs/UI_MIGRATION.md` (reforma moderna anterior) es historico. La fuente de verdad del estilo son `apps/desktop/src/lib/mc/` y `src/mc/*.css`.

## Prioridad de autenticacion

1. Offline Gaming funcional.
2. Microsoft anticipado arquitectonicamente desde Sprint 0.
3. Microsoft funcional en su sprint cuando la integracion externa sea viable.

"Offline primero" no es permiso para hardcodear el launcher alrededor de cuentas locales.

## Reglas de codigo

- `IdentityProvider` desde el primer sprint.
- `Instance` referencia `profile_id`, nunca tokens.
- Tokens fuera del frontend.
- `CredentialStore` existe antes de Microsoft.
- `FakeMicrosoftProvider` para contract tests.
- No duplicar launch logic por provider.
- No convertir Microsoft fallido en Local silenciosamente.
- No guardar credenciales en exports.

## Desarrollo

Trabaja por verticales grandes. En cada sprint:
1. Implementa.
2. Compila (`cargo clippy --all-targets`, `cargo test --workspace`).
3. Testea.
4. Ejecuta flujo real.
5. Repara.
6. Documenta cambios.
7. Continua.

## UX

Usuario objetivo no tecnico. No mostrar terminos internos salvo pantalla avanzada.
Estilo: escenario en pixeles de GUI con componentes `Mc*` y texto bitmap (nada de texto HTML visible, fuentes remotas ni datos simulados en la app). Ver `docs/UI_MIGRATION_MINECRAFT.md`.

## Stack

- **Core**: Rust estable + Tokio + Reqwest + Serde + thiserror + tracing.
- **Desktop**: Tauri 2.
- **Frontend**: Svelte + TypeScript + CSS propio (escenario Minecraft en px de GUI).
- **Persistencia**: JSON/TOML para config, secure storage del SO para secretos.

## Seguridad — prohibido

- Secretos hardcodeados.
- localStorage para tokens.
- Shell command concatenada para launch.
- Theme code execution.
- ZIP traversal.
- Logs con tokens.
- Mirrors inseguros como fuente primaria.
- `sh -c` / `cmd /c` para launch.

## Estructura del proyecto

```
docs/spec/           especificacion tecnica (00-13)
docs/adr/            decisiones arquitectonicas
docs/UI_MIGRATION_MINECRAFT.md interfaz estilo Minecraft: reglas, componentes y estado
DisorderLauncherUI/  maqueta visual (solo referencia, no se importa)
docs/reference/      HTML de referencia visual
theme_examples/      JSON de temas de ejemplo
```

## Resultado esperado

Un launcher que funciona bien con Offline Gaming y cuyo nucleo no necesita rediseno para Microsoft.
