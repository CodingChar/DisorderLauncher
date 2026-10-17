# DisorderLauncher

Launcher de Minecraft Java ligero, modular, seguro y usable por personas con poca experiencia informatica.

**Stack**: Rust (Tauri 2) + Svelte + TypeScript

**Estado**: Fase de especificacion — no hay codigo fuente todavia.

---

## Principio central

**Offline Gaming se implementa primero y debe quedar completamente funcional. Microsoft se implementa despues, pero se anticipa arquitectonicamente desde el primer commit.**

- Offline no es un hack temporal que luego se reemplaza.
- `IdentityProvider` existe desde Sprint 0.
- Las instancias almacenan una referencia de perfil, no credenciales.
- Los tokens Microsoft nunca viven en frontend/localStorage.

---

## Estructura del repositorio

```
docs/
  spec/           especificacion tecnica (14 documentos, 00-13)
  adr/            decisiones arquitectonicas
  reference/      HTML de referencia visual
theme_examples/   JSON de temas de ejemplo
.claude/          reglas para Claude Code
```

## Orden de lectura de la especificacion

1. [`00_MASTER.md`](docs/spec/00_MASTER.md) — mision, stack, principios, MVP
2. [`01_BOOTSTRAP_FROM_ZERO.md`](docs/spec/01_BOOTSTRAP_FROM_ZERO.md) — repo, workspace, Tauri, CI
3. [`02_ARCHITECTURE.md`](docs/spec/02_ARCHITECTURE.md) — capas, IPC, eventos, flujos tecnicos
4. [`03_IDENTITY_AUTH_OFFLINE_FIRST.md`](docs/spec/03_IDENTITY_AUTH_OFFLINE_FIRST.md) — Offline Gaming
5. [`04_MICROSOFT_AUTH_FUTURE.md`](docs/spec/04_MICROSOFT_AUTH_FUTURE.md) — Microsoft OAuth futuro
6. [`05_MINECRAFT_ENGINE.md`](docs/spec/05_MINECRAFT_ENGINE.md) — install, launch, Java, repair
7. [`06_INSTANCES_STORAGE.md`](docs/spec/06_INSTANCES_STORAGE.md) — instancias y almacenamiento
8. [`07_LOADERS_MODS_MODPACKS.md`](docs/spec/07_LOADERS_MODS_MODPACKS.md) — Fabric, Forge, mods
9. [`08_VISUAL_DESIGN_SYSTEM.md`](docs/spec/08_VISUAL_DESIGN_SYSTEM.md) — design system y UI
10. [`09_THEME_SYSTEM.md`](docs/spec/09_THEME_SYSTEM.md) — temas JSON
11. [`10_SKIN_SYSTEM.md`](docs/spec/10_SKIN_SYSTEM.md) — skins
12. [`11_SECURITY_PRIVACY.md`](docs/spec/11_SECURITY_PRIVACY.md) — seguridad y privacidad
13. [`12_TESTING_QA.md`](docs/spec/12_TESTING_QA.md) — testing y QA
14. [`13_SPRINTS_AND_ROADMAP.md`](docs/spec/13_LONG_SPRINTS.md) — sprints, dependencias y roadmap
15. [`ADR-001`](docs/adr/ADR-001_stack_and_references.md) — decisiones de stack y referencias
16. [`Referencia visual`](docs/reference/disorderlauncher_reference.html) — HTML de diseno

## Regla de autoridad

Si dos documentos se contradicen, prevalece `00_MASTER.md`. Despues prevalece el documento especifico del subsistema.

## Restriccion de producto

El perfil Offline Gaming debe servir para uso local/offline compatible, desarrollo y entornos que admitan identidades locales. No debe representarse como una cuenta Microsoft, falsificar ownership, ni intentar sustituir autenticacion de servicios online que exijan una sesion valida.
