# 17 — Flujos técnicos

## Offline launch

```text
Play
 -> Instance
 -> profile_id
 -> AccountRecord(Local)
 -> LocalIdentityProvider
 -> ResolvedIdentity
 -> VersionDescriptor
 -> Java
 -> LaunchPlan
 -> Process
```

## Microsoft launch futuro

```text
Play
 -> Instance
 -> profile_id
 -> AccountRecord(Microsoft)
 -> MicrosoftIdentityProvider
    -> cached session valid?
       yes -> ResolvedIdentity
       no  -> refresh token
              -> Microsoft token
              -> Xbox
              -> XSTS
              -> Minecraft Services
              -> profile/ownership
              -> ResolvedIdentity
 -> same VersionDescriptor
 -> same Java
 -> same LaunchPlan builder
 -> same ProcessRunner
```

La mitad inferior del flujo es idéntica. Esa es la razón principal para anticipar Microsoft en el diseño.

## Account lifecycle

```text
Create Local
 -> LocalReady

Connect Microsoft
 -> Authorizing
 -> Authenticated
 -> RefreshRequired
 -> Authenticated
 or Expired
```

## Failure boundary

Errores de identidad no deben corromper:
- instancia;
- assets;
- libraries;
- mods;
- saves.

## Offline availability

```text
identity ready?
 + installation complete?
 + Java available?
 = can launch without network
```

## Microsoft without network

No inventar una sesión nueva.

Si existe identidad cacheada válida para una función offline admitida por la política del producto, representarla con estado explícito; si no, pedir reconexión.
