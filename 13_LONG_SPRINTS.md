# 13 — Sprints largos

## Sprint 0 — Fundaciones + contratos que evitan reescritura

Entregar:
- monorepo/workspace;
- Tauri/Svelte;
- CI;
- storage;
- logging;
- error taxonomy;
- `IdentityProvider`;
- `AccountRecord`;
- `AuthState`;
- `CredentialStore`;
- LocalProvider skeleton;
- MicrosoftProvider feature-gated skeleton;
- provider contract tests;
- CSS tokens;
- base navigation;
- Microsoft feasibility spike documentado.

El spike Microsoft debe probar hasta donde permita la app registration, sin retrasar Offline Gaming.

Criterio:
arquitectura compila con Local y FakeMicrosoft intercambiables.

---

## Sprint 1 — Offline Gaming + Vanilla end-to-end

Entregar:
- creación de perfil local;
- validación;
- UUID local compatible;
- default account;
- account picker;
- version manifest;
- InstallPlan;
- downloader;
- client;
- libraries;
- assets;
- natives;
- Java;
- LaunchPlan;
- local identity resolution;
- launch;
- logs;
- network/offline detection;
- repair mínimo;
- tests.

Criterio:
desde instalación limpia, usuario crea perfil local, instala Vanilla y juega.

---

## Sprint 2 — Instancias + UX de producto

Entregar:
- create/edit/delete/duplicate;
- per-instance profile binding;
- RAM auto;
- Java override;
- repair completo;
- responsive;
- settings;
- error dialogs;
- accessibility.

Criterio:
usuario no técnico administra varias instancias sin tocar filesystem.

---

## Sprint 3 — Fabric + Mods + Modrinth

Entregar:
- Fabric;
- mods;
- dependencies;
- enable/disable;
- Modrinth;
- filters;
- updates;
- rollback básico.

---

## Sprint 4 — NeoForge + Forge + Quilt

Entregar:
- providers;
- installer/processors;
- inheritance;
- compatibility;
- diagnostics.

---

## Sprint 5 — Modpacks

Entregar:
- `.mrpack`;
- formato interno;
- import;
- export;
- overrides;
- rollback;
- progress.

---

## Sprint 6 — Microsoft product implementation

Precondición:
- app registration feasibility conocida.

Entregar:
- Authorization Code + PKCE;
- system browser;
- callback loopback;
- secure store;
- refresh;
- Xbox;
- XSTS;
- Minecraft Services;
- ownership/profile;
- account picker;
- switching;
- logout;
- error UX;
- contract tests;
- E2E donde sea viable.

Si Minecraft Services rechaza la app registration:
- no falsear flujo;
- mantener feature deshabilitada;
- documentar blocker;
- Offline Gaming continúa siendo completamente operativo.

---

## Sprint 7 — Themes v1

- default themes;
- JSON import;
- schema;
- selector;
- preview;
- fallback.

---

## Sprint 8 — Skins library

- import;
- validation;
- local profile link;
- preview;
- model.

---

## Sprint 9 — Skin editor

- canvas;
- tools;
- undo/redo;
- 3D;
- export;
- profile integration.

---

## Sprint 10 — Hardening y release

- packaging;
- updater;
- migrations;
- recovery;
- performance;
- QA matrix;
- security audit;
- release docs.
