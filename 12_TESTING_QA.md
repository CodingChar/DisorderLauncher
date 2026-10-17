# 12 — Testing y QA

## Pirámide

### Unit
- metadata parsing;
- rules;
- UUID local;
- account serialization;
- provider registry;
- theme schema;
- skin validation;
- Maven paths;
- arguments.

### Contract
Cada `IdentityProvider` debe pasar el mismo contrato:

- kind estable;
- resolve;
- refresh;
- logout;
- error mapping;
- no secret leakage.

Providers de test:
- LocalProvider real;
- FakeMicrosoftProvider.

### Integration
- account repository;
- credential store mock;
- Vanilla install;
- launch plan;
- repair;
- Java selection;
- Fabric.

### E2E
Offline-first:

```text
fresh install
 -> create local account
 -> create Vanilla instance
 -> install
 -> launch
 -> exit
 -> relaunch app
 -> launch again
```

## Microsoft tests anticipados

Desde Sprint 0:
- UI state tests;
- provider substitution;
- secure-store interface tests.

En sprint Microsoft:
- state mismatch;
- callback timeout;
- user cancel;
- token refresh;
- invalid_grant;
- network failure;
- XSTS failure;
- Minecraft Services app rejection;
- ownership unavailable.

## Offline network matrix

| Identidad | Archivos completos | Internet | Resultado |
|---|---:|---:|---|
| Local | Sí | No | Jugar |
| Local | No | No | Error claro |
| Local | No | Sí | Descargar |
| Microsoft | Sí | No | Depende de estado cacheado y política |
| Microsoft | No | No | No instalar |

## Fault injection

- partial download;
- hash mismatch;
- disk full;
- permission denied;
- corrupt JSON;
- invalid theme;
- invalid skin;
- Java removed after detection.

## Performance

Medir:
- cold start;
- idle memory;
- CPU idle;
- install throughput;
- UI responsiveness.

## Acceptance

Ningún sprint se cierra con:
- botones muertos;
- TODO crítico;
- token log;
- test rojo;
- build roto.
