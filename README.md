# DisorderLaucher — Engineering Master Package

Este paquete es la especificación de autoridad para construir DisorderLaucher desde cero.

## Principio central

**Offline Gaming se implementa primero y debe quedar completamente funcional. Microsoft se implementa después, pero se anticipa arquitectónicamente desde el primer commit.**

Esto significa:

- Offline no es un hack temporal que luego se reemplaza.
- Microsoft no se incrusta directamente en la UI ni en el motor de launch.
- `IdentityProvider` existe desde Sprint 0.
- Las instancias almacenan una referencia de perfil, no credenciales.
- El motor de launch recibe una identidad normalizada.
- Los tokens Microsoft nunca viven en frontend/localStorage.
- La UI diferencia claramente perfil local, cuenta Microsoft conectada, sesión expirada y modo sin conexión.

## Orden de lectura obligatorio

1. `00_MASTER.md`
2. `01_BOOTSTRAP_FROM_ZERO.md`
3. `02_ARCHITECTURE.md`
4. `03_IDENTITY_AUTH_OFFLINE_FIRST.md`
5. `04_MICROSOFT_AUTH_FUTURE.md`
6. `05_MINECRAFT_ENGINE.md`
7. `06_INSTANCES_STORAGE.md`
8. `07_LOADERS_MODS_MODPACKS.md`
9. `08_VISUAL_DESIGN_SYSTEM.md`
10. `09_THEME_SYSTEM.md`
11. `10_SKIN_SYSTEM.md`
12. `11_SECURITY_PRIVACY.md`
13. `12_TESTING_QA.md`
14. `13_LONG_SPRINTS.md`
15. `14_FUTURE_ROADMAP.md`
16. `15_AGENT_EXECUTION_PROMPT.md`
17. `16_REFERENCES.md`
18. `reference/disorderlaucher_reference.html`

## Regla de autoridad

Si dos documentos se contradicen, prevalece `00_MASTER.md`. Después prevalece el documento específico del subsistema.

## Restricción de producto

El perfil Offline Gaming debe servir para uso local/offline compatible, desarrollo y entornos que admitan identidades locales. No debe representarse como una cuenta Microsoft, falsificar ownership, ni intentar sustituir autenticación de servicios online que exijan una sesión válida.
