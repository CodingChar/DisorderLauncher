# 03 — Identidad: Offline Gaming primero

## 1. Objetivo

Offline Gaming es la primera implementación real de `IdentityProvider`.

Debe ser estable por sí sola y no depender de Microsoft.

## 2. Qué es un perfil local

Es una identidad administrada por DisorderLauncher:

```rust
pub struct LocalAccountData {
    pub display_name: String,
    pub local_uuid: Uuid,
    pub skin_id: Option<String>,
}
```

Persistida dentro de `AccountRecord`.

No contiene:
- password;
- access token;
- refresh token;
- Xbox identity;
- entitlement claim.

## 3. AccountRecord común

```rust
pub struct AccountRecord {
    pub id: String,
    pub provider: IdentityProviderKind,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub provider_data: ProviderAccountData,
}
```

```rust
pub enum ProviderAccountData {
    Local(LocalAccountData),
    Microsoft(MicrosoftAccountMetadata),
}
```

Desde Sprint 0 se define el enum completo aunque Microsoft todavía no esté habilitado.

## 4. UUID local

Para compatibilidad con entornos vanilla que utilizan el esquema clásico de identidad offline, implementar el algoritmo compatible con `UUID.nameUUIDFromBytes("OfflinePlayer:" + username)` de Java, con tests de vectores conocidos.

Debe estar encapsulado en `identity-local`.

No reutilizar este UUID para representar una cuenta Microsoft.

## 5. Resolución

```rust
LocalIdentityProvider::resolve_identity()
```

produce una `ResolvedIdentity` local.

No debe crear ni simular credenciales Microsoft.

El adaptador de launch local debe suministrar únicamente los campos no secretos que el cliente compatible requiera para una sesión local.

## 6. Estado

Perfil local válido:

```text
LocalReady
```

No usar:

```text
loggedIn = true
```

porque esa semántica causará confusión cuando exista Microsoft.

## 7. UI inicial

Primera ejecución:

```text
Bienvenido

¿Cómo quieres llamarte?
[ David                 ]

[ Crear perfil ]
```

Después:

```text
Perfil
David
Offline Gaming

[ Jugar ]
[ Cambiar skin ]
[ Administrar perfiles ]
```

No mostrar conceptos de OAuth.

## 8. Validación de nombres

Definir política explícita:
- longitud;
- caracteres soportados;
- trimming;
- unicidad visual;
- normalización.

No usar el nombre como ID primario interno.

## 9. Persistencia

Ejemplo:

```json
{
  "id": "acc_local_a91...",
  "provider": "local",
  "label": "David",
  "providerData": {
    "type": "local",
    "displayName": "David",
    "localUuid": "..."
  }
}
```

## 10. Instancias

La instancia guarda:

```json
{
  "profileId": "acc_local_a91..."
}
```

No guarda username duplicado.

Al cambiar el nombre del perfil, las instancias siguen funcionando porque referencian el ID.

## 11. Offline real

Distinguir:
- launcher sin internet;
- identidad local;
- archivos de Minecraft ya instalados.

Una instancia solo puede arrancar sin red si todos sus recursos requeridos ya están disponibles.

La UI debe mostrar:
- “Listo para jugar sin conexión”
- o “Necesita descargar archivos”.

## 12. Skins

La skin local pertenece al perfil local.

El launcher puede:
- guardar;
- editar;
- previsualizar;
- asociar.

La presentación de esa skin dentro del juego depende del mecanismo compatible de la instancia/entorno. No debe afirmarse que fue registrada en servicios Microsoft.

## 13. Límites

Offline Gaming no debe:
- hacerse pasar por Microsoft;
- acceder a servicios que requieran sesión Microsoft válida;
- intentar superar validaciones de servidores online-mode;
- falsificar entitlement.

## 14. Tests obligatorios

- creación de perfil;
- nombres inválidos;
- UUID estable;
- rename sin cambio de `account.id`;
- persistencia;
- resolución;
- launch con fake instance;
- múltiples perfiles;
- cambio de default;
- skin association;
- modo sin red con instalación completa/incompleta.
