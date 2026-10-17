import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

// Misma politica que `app.security.devCsp` de tauri.conf.json: asi las violaciones aparecen
// tambien al desarrollar en el navegador (con el backend simulado).
const DEV_CSP = [
  "default-src 'self'",
  "script-src 'self'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "font-src 'self'",
  "connect-src 'self' ws://localhost:5173 http://localhost:5173 ipc: http://ipc.localhost",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
].join('; ')

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  server: { headers: { 'Content-Security-Policy': DEV_CSP } },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
})
