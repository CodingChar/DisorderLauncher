# Publicar una version de DisorderLauncher

## Requisitos previos

| Plataforma | Herramientas | Firma |
|---|---|---|
| Windows | Rust estable, Node 22, WebView2 (incluido en Windows 10/11 actuales), NSIS (lo descarga `tauri build`) | Certificado de firma de codigo (Authenticode). Sin firmar, SmartScreen avisa al usuario. |
| macOS | Xcode CLT | Cuenta de Apple Developer: firma + notarizacion. |
| Linux | `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`, `libssl-dev` | Opcional (AppImage/deb). |

## Construir

> Verificado en Windows 10: `npx tauri build --bundles msi` genera `DisorderLauncher_0.1.0_x64_en-US.msi` (6 MiB; ~15 min la primera vez por la compilacion release).
> El empaquetado NSIS fallo en este equipo con `os error 32` al recrear la carpeta de herramientas de Tauri (`%LOCALAPPDATA%\tauri\NSIS`, archivo bloqueado): es del entorno, no del proyecto; borrar esa carpeta o reintentar tras reiniciar suele bastar.

```bash
cd apps/desktop
npm ci
npm run tauri build            # instaladores de la plataforma actual en src-tauri/target/release/bundle/
npx tauri build --bundles nsis # solo el instalador NSIS en Windows
```

La CSP de produccion esta en `src-tauri/tauri.conf.json` (`app.security.csp`). No relajarla sin actualizar `docs/SECURITY_AUDIT.md`.

## Antes de etiquetar

1. `cargo clippy --workspace --all-targets -- -D warnings` y `cargo test --workspace`.
2. `npm run check && npm test && npm audit --omit=dev`.
3. Pruebas reales (necesitan internet; ver `docs/QA_MATRIX.md`): Vanilla, Fabric, modpack.
4. Subir la version en `Cargo.toml` (`[workspace.package]`), `apps/desktop/src-tauri/tauri.conf.json` y `apps/desktop/package.json`.
5. Revisar que `docs/IPC.md` coincide con los comandos registrados en `src-tauri/src/lib.rs`.

## Migraciones de datos

Cada archivo persistente lleva `schemaVersion`. Al cambiar un formato:

1. Subir la version y escribir una migracion determinista de la anterior a la nueva.
2. Conservar copia `.bak-v<N>` antes de reescribir y avisar solo si algo falla.
3. Los archivos de una version **mas nueva** nunca se modifican (se informa al usuario).

Ejemplo vigente: cuentas v0 (claves `snake_case`) → v1 (`camelCase`) en `crates/account-storage/src/file_repository.rs`.

## Auto-actualizacion (bloqueado por infraestructura)

El plugin oficial (`tauri-plugin-updater`) exige dos cosas que el proyecto aun no tiene y que **no se deben simular**:

1. **Un par de claves de firma de actualizaciones** (`tauri signer generate`). La clave privada vive fuera del repositorio (secretos de CI) y la publica se pone en `plugins.updater.pubkey`.
2. **Un endpoint que sirva el manifiesto** (`latest.json`) y los artefactos firmados (GitHub Releases o similar), accesible por HTTPS.

Cuando existan: anadir el plugin, declarar `pubkey` y `endpoints` en `tauri.conf.json`, exponer un comando `updates_check`/`updates_install` en `src-tauri`, y mostrar en Ajustes "Buscar actualizaciones". Hasta entonces los usuarios actualizan descargando el instalador nuevo; los datos se conservan porque viven en el directorio de datos de la app y se migran solos.

## Datos del usuario

Directorio de datos: `app_data_dir` de Tauri (en Windows `%APPDATA%\com.disorderlauncher.app`). Desinstalar la app **no** borra ese directorio. Contiene perfiles, instancias, skins, temas y ajustes; los secretos de Microsoft estan en el almacen seguro del sistema (Credential Manager / Keychain / Secret Service), no en archivos.

## Publicar en GitHub y en la web

La landing (`landing/`) siempre ofrece la ultima release: los botones apuntan a `releases/latest/download/DisorderLauncher_x64-setup.exe` y `landing/script.js` lee `releases/latest` para mostrar version, tamano y SHA-256 (sin depender de ello para descargar). Por eso **cada release debe llevar dos copias del instalador**:

```bash
gh release create vX.Y.Z "DisorderLauncher_X.Y.Z_x64-setup.exe" "DisorderLauncher_x64-setup.exe" --latest --notes-file notas.md
```

`DisorderLauncher_x64-setup.exe` es el mismo archivo con nombre sin version. La landing solo cambia si se edita `landing/`; entonces lanzar el workflow "Landing (GitHub Pages)".
