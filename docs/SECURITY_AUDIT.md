# Auditoria de seguridad — Sprint 10

Alcance: todo el workspace Rust, el frontend y la configuracion de Tauri, contra `docs/spec/11_SECURITY_PRIVACY.md` y las reglas de `.claude/CLAUDE.md`. Fecha: 2026-10-17. Metodo: revision de codigo, busqueda dirigida (`unsafe`, `Command::new`, logs con secretos, rutas construidas con datos externos), pruebas automaticas y comprobaciones en el navegador.

## Hallazgos corregidos

| # | Hallazgo | Severidad | Correccion | Prueba |
|---|---|---|---|---|
| 1 | **Path traversal por id de cuenta**: el `id` recibido por IPC se usaba como nombre de archivo (`accounts_remove("../x")` podia borrar un `.json` fuera de `accounts/`). `_default.txt` manipulado tambien se aceptaba. | Alta | Ids validados (`[A-Za-z0-9_-]`, ≤ 64) en todo acceso; predeterminado invalido se ignora. | `account_ids_cannot_escape_the_directory` |
| 2 | CSP de Tauri en `null` (sin restricciones). | Alta | CSP estricta en produccion y `devCsp`: `default-src 'self'`, sin scripts inline, `img-src 'self' data:`, `connect-src` solo IPC, `object-src 'none'`, `frame-ancestors 'none'`. El servidor de desarrollo envia la misma politica. | Recorrido completo de la UI sin violaciones en consola |
| 3 | Descargas de Minecraft solo exigian HTTPS + hash, sin allowlist de dominios. | Media | `DownloadManager::with_allowed_host_suffixes`; el instalador solo acepta `mojang.com` y `minecraft.net` (subdominios, no dominios parecidos). | `host_allowlist_accepts_subdomains_but_not_lookalikes` + E2E real |
| 4 | Redirecciones HTTP→HTTP posibles en el cliente HTTP. | Media | Politica de redireccion: prohibido bajar a `http`, maximo 5 saltos. | — (comportamiento de `reqwest`) |
| 5 | Un archivo de cuenta danado hacia fallar la carga de **todos** los perfiles. | Media | Cuarentena (`.corrupt`), el resto sigue; formatos futuros no se tocan; aviso al usuario. | `corrupt_and_future_files_do_not_hide_healthy_accounts` |
| 6 | Extraccion de Java en Windows con `powershell Expand-Archive` e interpolacion de rutas (viola "no shell concatenada") y sin verificar el runtime. | Alta | Extraccion en proceso con proteccion ZIP traversal; SHA-256 publicado por Adoptium obligatorio. | `sha256_is_verified`, `traversal_entries_are_rejected…` |
| 7 | Dependencia de desarrollo `vitest 3` con 3 avisos de `npm audit` (2 criticos). | Baja (solo dev) | Actualizada a vitest 5: `npm audit` = 0. | `npm audit` |

## Controles verificados (sin cambios necesarios)

- **Procesos**: unicos `Command::new`: `java` (argv directo, sin shell), `java -version` y `tar` (argumentos separados). Sin `sh -c`/`cmd /c`. Entorno del juego con allowlist (no hereda secretos). Salida del juego redactada.
- **Secretos**: `Secret` (sin `Debug`, `zeroize`), `GameSessionMaterial` redacta el token en `Debug`, `LaunchPlan` no imprime argumentos. Refresh token solo en el almacen seguro del SO; `AccountRecord` no tiene campos de token (probado con registros en disco). Los `tracing` no registran tokens ni la URL de autorizacion.
- **OAuth**: PKCE S256, `state` aleatorio validado, callback solo desde loopback, un solo uso, timeout, cliente publico sin secreto, endpoints solo HTTPS (o loopback en pruebas).
- **ZIP**: `enclosed_name`, sin enlaces simbolicos, limite de bytes y de entradas (natives, Java, `.mrpack`, jars de mods con limite de metadatos).
- **Rutas externas**: ids de instancia, skin y tema validados; nombres de mod y rutas de modpack normalizados; `overrides/` no puede pisar `.disorder/`.
- **Hashes**: SHA-1 en Minecraft/mods/modpacks, SHA-256 en Java; descriptores de version verificados contra el manifiesto.
- **Frontend**: sin `localStorage`; sin `innerHTML`/`{@html}`; los temas solo escriben variables CSS tras validacion en Rust; skins como data URL de PNG ya normalizado.
- **Permisos de Tauri**: solo `core:default`. Los dialogos y el navegador se invocan **desde Rust**; el frontend no puede elegir rutas ni abrir URLs.
- **Codigo `unsafe`**: ninguno en el workspace.
- **Dependencias Rust**: sin `cargo-audit` disponible en este entorno; ver pendientes.

## Pendientes y riesgos aceptados

1. **`cargo audit`/`cargo deny` en CI**: no se pudo ejecutar localmente. Anadir a CI (`rustsec/audit-check`).
2. **Allowlist de redirecciones**: un host permitido podria redirigir a otro; el hash/checksum protege la integridad, pero no se restringe el destino final.
3. **Microsoft sin verificar contra los servidores reales** (no hay app registrada). Probado con un servidor falso que imita el contrato.
4. **Firma de codigo y actualizador**: requieren certificado y clave de firma del proyecto (ver `docs/RELEASE.md`).
5. **`style-src 'unsafe-inline'`**: necesario para los estilos que Svelte aplica por atributo; los scripts siguen sin inline.
6. **El editor de skins** se maneja con puntero: no hay alternativa de teclado para dibujar pixel a pixel (limitacion de accesibilidad registrada).
7. **Mods**: el launcher no puede garantizar que un mod sea seguro (spec 11 §9); se muestran origen y version, y los archivos externos solo se validan como jar de Fabric.
