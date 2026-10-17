# DisorderLauncher — aplicación de escritorio

Tauri 2 + Svelte 5 + TypeScript. La lógica vive en `crates/launcher-core`; aquí solo hay interfaz e IPC (`src-tauri`).

```bash
npm install
npm run dev        # navegador con backend simulado (src/lib/api/mock.ts)
npm run tauri dev  # ventana nativa con el backend real
npm run check && npm test && npm run build
```

**Interfaz**: el estilo sale de `src/app.css` (tokens + clases globales) y sigue la maqueta `../../DisorderLauncherUI`.
Antes de tocar la UI lee [`docs/UI_MIGRATION.md`](../../docs/UI_MIGRATION.md): reglas (sin fuentes remotas, sin datos simulados,
texto para personas no técnicas) y mapa pantalla ↔ maqueta.
