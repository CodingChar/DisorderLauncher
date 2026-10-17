# 04 — Microsoft: implementación futura anticipada

## 1. Principio

Microsoft se implementa después de Offline Gaming, pero no puede ser una remodelación del launcher.

Debe enchufarse como otro `IdentityProvider`.

## 2. Flujo previsto

Arquitectura objetivo:

```text
System browser
 -> Microsoft OAuth authorization code + PKCE
 -> Microsoft token
 -> Xbox user authentication
 -> XSTS
 -> Minecraft Services authentication
 -> Minecraft profile / ownership state
 -> MicrosoftIdentityProvider
 -> ResolvedIdentity
```

## 3. Cliente desktop público

DisorderLaucher es una aplicación desktop y debe tratarse como public client.

Implicación:
- no se puede proteger un client secret dentro del binario;
- usar Authorization Code + PKCE;
- abrir navegador del sistema;
- no usar password grant;
- no incrustar credenciales del usuario.

Si algún paso futuro requiere un secreto confidencial, ese paso no debe resolverse hardcodeando el secreto: requeriría una arquitectura backend apropiada.

## 4. Callback

Preferencia:
- loopback `127.0.0.1`;
- puerto dinámico cuando el registro lo permita;
- timeout corto;
- servidor de callback de un solo uso.

Obligatorio:
- `state` aleatorio;
- PKCE S256;
- code verifier criptográficamente aleatorio;
- validar `state`;
- invalidar sesión de login después del callback;
- no aceptar callback desde interfaces externas.

## 5. CredentialStore

Interfaz desde Sprint 0:

```rust
#[async_trait]
pub trait CredentialStore {
    async fn put_secret(&self, key: &str, secret: SecretString) -> Result<()>;
    async fn get_secret(&self, key: &str) -> Result<Option<SecretString>>;
    async fn delete_secret(&self, key: &str) -> Result<()>;
}
```

Implementación:
- Windows Credential Manager / DPAPI abstraction;
- macOS Keychain;
- Linux Secret Service/keyring.

Fallback inseguro:
- no silencioso;
- deshabilitar persistencia de refresh token o pedir consentimiento explícito para una alternativa adecuadamente cifrada.

## 6. Qué persistir

Seguro:
- refresh token: secure store;
- access token: memoria/caché efímera;
- XSTS/Minecraft tokens: memoria;
- account id/name: filesystem normal;
- token expiry metadata: filesystem normal si no contiene secreto.

## 7. MicrosoftAccountMetadata

```rust
pub struct MicrosoftAccountMetadata {
    pub minecraft_uuid: Option<String>,
    pub minecraft_name: Option<String>,
    pub secure_refresh_token_key: Option<String>,
    pub last_successful_auth: Option<DateTime<Utc>>,
}
```

Nunca serializar tokens dentro de este struct.

## 8. Estado Microsoft

```text
MicrosoftDisconnected
MicrosoftAuthorizing
MicrosoftAuthenticated
MicrosoftRefreshRequired
MicrosoftOfflineCachedIdentity
MicrosoftExpired
Error
```

`MicrosoftOfflineCachedIdentity` significa que existe información de identidad previamente validada pero no una sesión online vigente. No convertirla en perfil local automáticamente.

## 9. Refresh

Reglas:
- single-flight refresh por cuenta;
- refrescar antes de expiración;
- distinguir `invalid_grant` de errores de red;
- un error transitorio no borra el refresh token;
- revocación/invalid_grant sí obliga a reautenticar.

## 10. Ownership / entitlement

El provider Microsoft debe verificar el estado necesario antes de ofrecer funciones que dependan de ownership.

La UI no debe ocultar errores de ownership.

## 11. Riesgo de app registration

Existe un riesgo externo importante: una app registration propia puede completar OAuth/Xbox/XSTS y aun así ser rechazada por Minecraft Services si la aplicación no está habilitada/aceptada.

Por ello hay dos hitos:

### Spike técnico temprano
- registrar app;
- probar PKCE;
- validar scopes;
- comprobar callback;
- comprobar Xbox/XSTS;
- verificar respuesta de Minecraft Services.

### Implementación de producto posterior
- UI completa;
- secure store;
- refresh;
- ownership;
- account switching;
- skin/profile integration.

El spike temprano no bloquea el MVP Offline Gaming.

## 12. UI anticipada

Desde el primer diseño, Account Picker debe soportar:

```text
David
Offline Gaming

Microsoft
Conectar cuenta
```

Pero el botón Microsoft puede estar:
- oculto por feature flag;
- o marcado como futuro en builds de desarrollo.

## 13. Tests de contrato desde Sprint 0

Crear `FakeMicrosoftProvider` para probar que:
- una instancia no depende de LocalProvider;
- `LaunchService` acepta cualquier provider;
- estados de auth se renderizan correctamente;
- errores de provider no rompen instancias.

## 14. No hacer

- no copiar client IDs de otros launchers;
- no incluir client secret;
- no guardar tokens en JS/localStorage;
- no mezclar refresh token con export de perfil;
- no convertir automáticamente una cuenta Microsoft caída en Local sin informar al usuario.
