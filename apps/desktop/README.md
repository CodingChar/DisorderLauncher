# DisorderLauncher — aplicación de escritorio

Tauri 2 + Svelte 5 + TypeScript. La lógica vive en `crates/launcher-core`; aquí solo hay interfaz e IPC (`src-tauri`).

```bash
npm install
npm run dev        # navegador con backend simulado (src/lib/api/mock.ts)
npm run tauri dev  # ventana nativa con el backend real
npm run check && npm test && npm run build
```

**Interfaz**: estilo de los menús de Minecraft 1.7.10 (escenario en píxeles de GUI, texto con la fuente bitmap, componentes `src/lib/mc/`,
CSS en `src/mc/`). Antes de tocar la UI lee [`docs/UI_MIGRATION_MINECRAFT.md`](../../docs/UI_MIGRATION_MINECRAFT.md): reglas (sin fuentes
remotas, sin datos simulados, texto para personas no técnicas), componentes, mapa de pantallas y cómo auditarlas en el navegador
(`window.__mcAudit()`, `window.__mcMatrix({ seed: true })`).

## Créditos

Texturas y fuente inspiradas en Minecraft, © Mojang. DisorderLauncher no está afiliado a Mojang ni a Microsoft.
