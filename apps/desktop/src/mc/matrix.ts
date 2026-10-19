// Solo DEV (con el backend simulado): recorre todas las pantallas y subpantallas del modo Minecraft con datos
// extremos y devuelve los problemas del auditor. `window.__mcMatrix({ seed: true })` en la consola del navegador.

import { api } from '../lib/api'
import { accounts } from '../lib/stores/accounts.svelte'
import { instances } from '../lib/stores/instances.svelte'
import { nav } from '../lib/stores/nav.svelte'
import { skins } from '../lib/stores/skins.svelte'
import { auditStage } from './audit'

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

function pngDataUrl(): string {
  const c = document.createElement('canvas')
  c.width = 64
  c.height = 64
  const ctx = c.getContext('2d')!
  ctx.fillStyle = '#c8a07f'
  ctx.fillRect(8, 8, 8, 8)
  ctx.fillRect(20, 20, 8, 12)
  return c.toDataURL('image/png')
}

/** Datos del mock con casos limite: nombres con ñ ¿ ¡ « » y de 64 caracteres, muchas instancias y varias skins. */
export async function seed(extra = 0) {
  if (accounts.list.length === 0) await accounts.createLocal('Alexandra')
  await accounts.createLocal('Dieciseis_chars_x').catch(() => {})
  const names = ['Mi mundo', 'Fabric con ñandúes ¿qué? «sí» ¡ya!', 'N'.repeat(64), 'Emoji 😀 sin glifo', 'Con\nsalto de línea']
  for (const [i, name] of names.entries()) await instances.create(name, i === 3 ? '1.21.4-forge-54.0.12' : '1.21.4', i % 2 ? 'fabric' : 'vanilla')
  for (let i = 0; i < extra; i++) await instances.create(`Instancia ${i + 1}`, '1.20.1', 'vanilla')
  for (const name of ['Steve', 'Una skin con un nombre larguísimo de cuarenta y ocho caracteres!!']) await api.skinsSave(null, name, pngDataUrl(), 'classic')
  await skins.load(true)
}

export async function runMatrix(options: { seed?: boolean; extra?: number } = {}): Promise<string[]> {
  if (options.seed) await seed(options.extra ?? 0)
  const res: string[] = []
  const lab = (prefix: string) => [...document.querySelectorAll<HTMLElement>('button, [role="slider"], [role="tab"]')].find((b) => (b.getAttribute('aria-label') ?? '').startsWith(prefix))
  const click = async (prefix: string) => {
    const b = lab(prefix) as HTMLButtonElement | undefined
    if (!b) return void res.push(`SIN BOTON: ${prefix}`)
    if (b.disabled) return void res.push(`DESHABILITADO: ${prefix}`)
    b.click()
    await sleep(350)
  }
  const check = async (name: string) => {
    await sleep(350)
    for (const p of auditStage()) res.push(`${name}: ${p.kind} | ${p.element} | ${JSON.stringify(p.rect)}`)
  }
  const go = async (view: Parameters<typeof nav.go>[0]) => {
    nav.go(view)
    await sleep(500)
  }
  const esc = async () => {
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await sleep(300)
  }

  await go('menu')
  await check('menu')
  await click('Ayuda y soporte')
  await check('menu/ayuda')
  await esc()
  await go('instances')
  await check('instances')
  await go('home')
  await check('home')
  await go('create')
  await check('create')
  await click('Versión de Minecraft')
  await check('create/version')
  await esc()
  await click('Icono')
  await check('create/icono')
  await esc()
  await click('Opciones avanzadas')
  await check('create/avanzadas')
  await click('Memoria')
  await check('create/avanzadas-memoria')
  await click('Java')
  await check('create/avanzadas-java')
  await esc()
  await click('Personalizado')
  await check('create/personalizado')
  await click('Versión instalada')
  await check('create/personalizado-version')
  await esc()
  await go('edit')
  await check('edit')
  await click('Más acciones')
  await check('edit/mas')
  await click('Eliminar')
  await check('edit/eliminar-dialogo')
  await esc()
  await esc()
  await click('Opciones avanzadas')
  await check('edit/avanzadas')
  await esc()
  await go('explore')
  await check('explore')
  await go('tools')
  await check('tools')
  await go('accounts')
  await check('accounts')
  await click('Añadir perfil')
  await check('accounts/nuevo')
  await esc()
  await click('Quitar este perfil')
  await check('accounts/quitar-dialogo')
  await esc()
  await go('settings')
  await check('settings')
  await click('Tema especial')
  await check('settings/temas')
  await esc()
  await go('skins')
  await check('skins')
  await click('Más opciones')
  await check('skins/mas')
  await click('Renombrar')
  await check('skins/renombrar')
  await esc()
  await esc()
  await go('skinEditor')
  await check('skinEditor')
  await click('Opciones del editor')
  await check('skinEditor/opciones')
  await esc()
  const fabric = instances.list.find((i) => i.loader === 'fabric')
  if (fabric) instances.select(fabric.id)
  await go('mods')
  await check('mods')
  await click('Buscar mods')
  await check('mods/buscar')
  await go('menu')
  return res
}

window.__mcMatrix = runMatrix
