# 11 — Seguridad y privacidad

## 1. Threat model

Amenazas:
- archivo de mod malicioso;
- ZIP traversal;
- theme malicioso;
- token theft;
- callback OAuth spoofing;
- log leakage;
- path injection;
- command injection;
- corrupted metadata;
- supply-chain URLs.

## 2. Offline Gaming

No requiere secretos.

Persistencia:
- account metadata normal.

No inventar campos de autenticación Microsoft.

## 3. Microsoft

Secretos:
- refresh token;
- access token;
- tokens derivados.

Reglas:
- secure storage;
- redaction;
- zeroize cuando sea práctico;
- no frontend;
- no clipboard;
- no logs.

## 4. OAuth

- PKCE S256.
- random state.
- system browser.
- loopback only.
- short-lived callback server.
- timeout.
- one-shot callback.
- TLS para endpoints remotos.

## 5. Desktop public client

Nunca hardcodear client secret.

Un binario distribuido no puede mantenerlo confidencial.

## 6. Process spawning

No usar:
```text
sh -c
cmd /c
```
para construir launch normal.

Usar argv.

## 7. Downloads

- HTTPS;
- allowlist de hosts por proveedor cuando sea viable;
- hashes;
- size limits;
- temp files.

## 8. Themes

Datos declarativos únicamente.

## 9. Mods

No afirmar que un mod es seguro solo porque proviene de un repositorio.

Mostrar fuente y hash cuando esté disponible.

## 10. Privacy

Por defecto:
- sin analytics;
- sin upload de logs;
- sin cloud sync.

Cualquier telemetría futura:
- opt-in;
- documentada;
- minimizada.

## 11. Exports

Sanitizar:
- tokens;
- paths personales;
- identifiers innecesarios.
