# 15 — Prompt maestro para agente

Eres el agente principal de DisorderLaucher.

## Lectura obligatoria

Lee TODOS los MD antes de modificar código.

La prioridad de autenticación es:

1. Offline Gaming funcional.
2. Microsoft anticipado arquitectónicamente desde Sprint 0.
3. Microsoft funcional en su sprint cuando la integración externa sea viable.

No interpretes “Offline primero” como permiso para hardcodear el launcher alrededor de cuentas locales.

## Reglas

- `IdentityProvider` desde el primer sprint.
- `Instance` referencia `profile_id`.
- tokens fuera del frontend.
- `CredentialStore` existe antes de Microsoft.
- FakeMicrosoftProvider para contract tests.
- no duplicar launch logic por provider.
- no convertir Microsoft fallido en Local silenciosamente.
- no guardar credenciales en exports.

## Desarrollo

Trabaja por verticales grandes.

En cada sprint:
1. implementa;
2. compila;
3. testea;
4. ejecuta flujo real;
5. repara;
6. documenta cambios;
7. continúa.

## UX

Usuario objetivo no técnico.

No mostrar términos internos salvo pantalla avanzada.

## Microsoft

Haz un feasibility spike temprano, pero no desplaces la prioridad de Offline Gaming.

Para OAuth desktop:
- public client;
- Authorization Code + PKCE;
- system browser;
- loopback callback;
- no client secret.

La integración Microsoft debe estar feature-gated hasta ser productivamente viable.

## Seguridad

Nunca:
- secretos hardcodeados;
- localStorage para tokens;
- shell command concatenada;
- theme code execution;
- ZIP traversal;
- logs con tokens;
- mirrors inseguros como fuente primaria.

## Resultado esperado

Un launcher que ya funciona bien con Offline Gaming y cuyo núcleo no necesita ser rediseñado para Microsoft.
