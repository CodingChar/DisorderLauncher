# 16 — Referencias y dependencias externas

## Microsoft OAuth / desktop

Microsoft recomienda Authorization Code con PKCE para aplicaciones desktop/native y señala que un public client no debe incluir un client secret dentro del binario.

- https://learn.microsoft.com/en-us/windows/apps/develop/security/oauth2
- https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow
- https://learn.microsoft.com/en-us/entra/msal/msal-authentication-flows

## Xbox services

- https://learn.microsoft.com/en-us/gaming/gdk/docs/services/fundamentals/s2s-auth-calls/service-authentication/live-xbox-live-authentication

## Riesgo observado en 2026 para launchers de terceros

Existen reportes recientes de app registrations que completan Microsoft OAuth, Xbox y XSTS pero reciben `Invalid app registration` al llegar a Minecraft Services. Tratar como riesgo externo y validarlo mediante un spike temprano.

- https://learn.microsoft.com/en-us/answers/questions/5989335/minecraft-services-returns-http-403-invalid-app-re
- https://learn.microsoft.com/en-us/answers/questions/5945628/minecraft-services-returns-invalid-app-registratio

## Minecraft metadata

Manifest utilizado por launchers para catálogo de versiones:

- https://piston-meta.mojang.com/mc/game/version_manifest_v2.json

## OpenLauncher — referencia arquitectónica revisada

Repositorio:
- https://github.com/CesarGarza55/OpenLauncher

Archivos relevantes:
- `src/lib/minecraftLauncher.js`
- `src/lib/microsoftAuth.js`
- `electron/services/minecraftInstaller.js`
- `electron/services/minecraftRunner.js`
- `electron/ipc/registerIpc.js`

Lecciones adoptadas:
- separación de catálogo e instalación;
- OAuth PKCE;
- secure storage;
- refresh deduplicado;
- identidad local independiente.

Lecciones a mejorar:
- evitar `loggedIn=true` para identidad local;
- usar UUID offline compatible con Java si se requiere compatibilidad;
- separar más fuertemente auth, launch e instalación;
- no mezclar fallback Microsoft con identidad local sin estado explícito.
