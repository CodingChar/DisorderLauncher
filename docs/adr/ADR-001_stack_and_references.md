# ADR-001 — Stack tecnologico y referencias

## Estado: Aceptado

## Contexto

DisorderLauncher necesita un stack que soporte: rendimiento, seguridad (tokens nunca en frontend), empaquetado desktop multiplataforma, y una UI moderna.

## Decision

### Core: Rust + Tokio
- Seguridad de memoria sin GC.
- Async nativo con Tokio.
- Tipado fuerte para errores de dominio.
- Ecosystem maduro para HTTP (reqwest), serialization (serde), crypto (sha1/sha2).

### Desktop: Tauri 2
- Binarios pequenos vs Electron.
- IPC tipado Rust <-> frontend.
- CSP restrictiva por defecto.
- Acceso controlado a filesystem, proceso, secure storage.

### Frontend: Svelte + TypeScript
- Bundle pequeno.
- Reactividad sin virtual DOM.
- TypeScript strict para contratos IPC.
- CSS propio tokenizado (sin framework UI pesado).

### Persistencia
- JSON/TOML para config y manifiestos.
- Secure storage del SO para secretos (keyring/Credential Manager/Keychain).
- SQLite solo si hay necesidad transaccional real.

## Referencia arquitectonica: OpenLauncher

Repositorio: https://github.com/CesarGarza55/OpenLauncher

### Lecciones adoptadas
- Separacion de catalogo e instalacion.
- OAuth PKCE.
- Secure storage.
- Refresh deduplicado.
- Identidad local independiente.

### Lecciones a mejorar
- Evitar `loggedIn=true` para identidad local.
- Usar UUID offline compatible con Java si se requiere compatibilidad.
- Separar mas fuertemente auth, launch e instalacion.
- No mezclar fallback Microsoft con identidad local sin estado explicito.

## Referencias externas

### Microsoft OAuth / desktop
- https://learn.microsoft.com/en-us/windows/apps/develop/security/oauth2
- https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow
- https://learn.microsoft.com/en-us/entra/msal/msal-authentication-flows

### Xbox services
- https://learn.microsoft.com/en-us/gaming/gdk/docs/services/fundamentals/s2s-auth-calls/service-authentication/live-xbox-live-authentication

### Riesgo de app registration (2026)
- https://learn.microsoft.com/en-us/answers/questions/5989335/minecraft-services-returns-http-403-invalid-app-re
- https://learn.microsoft.com/en-us/answers/questions/5945628/minecraft-services-returns-invalid-app-registratio

### Minecraft metadata
- https://piston-meta.mojang.com/mc/game/version_manifest_v2.json
