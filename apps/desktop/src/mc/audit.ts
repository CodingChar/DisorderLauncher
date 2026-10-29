import { nav } from '../lib/stores/nav.svelte'

// Auditor de maquetacion del modo Minecraft. Solo DEV: `window.__mcAudit()` devuelve la lista de problemas
// del DOM actual (texto fuera de su control, solapes, medidas no enteras, texto HTML visible...).
// Ver docs/UI_MIGRATION_MINECRAFT.md §Verificacion.

interface Box {
  l: number
  t: number
  r: number
  b: number
}

export interface AuditProblem {
  screen: string
  kind: string
  element: string
  rect: Box
  container?: Box
}

const CONTROLS = 'button:not(:disabled), input:not(:disabled), [role="slider"], [role="listbox"]'
const CONTAINERS = '.mc-button, .mc-field, .mc-slider, .mc-row, .mc-list, .mc-dialog-layer, .mc-more'
const MEASURED = '.mc-button, .mc-field, .mc-slider, .mc-list, .mc-label, .mc-row, .mc-viewport, .mc-progress, .mc-icon-cell, .mc-swatch'

const box = (el: Element): Box => {
  const r = el.getBoundingClientRect()
  return { l: r.left, t: r.top, r: r.right, b: r.bottom }
}
const visible = (el: Element) => {
  const r = el.getBoundingClientRect()
  return r.width > 0 && r.height > 0
}
// Con un dialogo abierto lo de debajo queda inerte: no se puede pulsar ni enfocar, asi que no se audita como control.
const live = (el: Element) => !el.closest('[inert]')
const round = (b: Box): Box => ({ l: Math.round(b.l * 10) / 10, t: Math.round(b.t * 10) / 10, r: Math.round(b.r * 10) / 10, b: Math.round(b.b * 10) / 10 })
const inside = (a: Box, c: Box, tol: number) => a.l >= c.l - tol && a.t >= c.t - tol && a.r <= c.r + tol && a.b <= c.b + tol
const area = (a: Box, c: Box) => Math.max(0, Math.min(a.r, c.r) - Math.max(a.l, c.l)) * Math.max(0, Math.min(a.b, c.b) - Math.max(a.t, c.t))
const describe = (el: Element) => {
  const label = el.getAttribute('aria-label') ?? el.textContent?.trim().slice(0, 30) ?? ''
  return `${el.tagName.toLowerCase()}.${[...el.classList].join('.')}${label ? ` "${label}"` : ''}`
}

export function auditStage(): AuditProblem[] {
  const stage = document.querySelector<HTMLElement>('.mc-stage')
  if (!stage) return []
  const s = parseFloat(getComputedStyle(stage).getPropertyValue('--s')) || 1
  const stageBox = box(stage)
  const screen = stage.querySelector('h1')?.textContent ?? '(sin titulo)'
  const problems: AuditProblem[] = []
  const add = (kind: string, el: Element, container?: Box) => problems.push({ screen, kind, element: describe(el), rect: round(box(el)), container: container && round(container) })
  const shadow = s // 1 px de GUI de sombra a la derecha y abajo
  const tol = 0.51

  const canvases = [...stage.querySelectorAll<HTMLCanvasElement>('.bitmap-label canvas')].filter((c) => visible(c) && !c.closest('.splash'))
  const liveCanvases = canvases.filter(live)

  // 1. Texto dentro de su control.
  for (const canvas of canvases) {
    const holder = canvas.closest(CONTAINERS)
    if (!holder) continue
    const c = box(holder)
    const t = box(canvas)
    t.r -= shadow
    t.b -= shadow
    if (!inside(t, c, tol)) add('texto-fuera-de-su-control', canvas, c)
  }

  // 2. Nada fuera de la pantalla (las filas fuera de la banda de una lista estan recortadas a proposito).
  const clipped = (el: Element) => {
    const list = el.closest('.mc-list')
    return !!list && area(box(el), box(list)) === 0
  }
  for (const el of stage.querySelectorAll(`${MEASURED}, .bitmap-label canvas`)) {
    if (!visible(el) || el.closest('.splash') || clipped(el)) continue
    if (el.closest('.mc-list') && el !== el.closest('.mc-list')) continue // el recorte de la lista lo gestiona su banda
    if (!inside(box(el), stageBox, tol)) add('fuera-de-pantalla', el, stageBox)
  }

  // 3. Solapes entre controles y entre textos.
  const controls = [...stage.querySelectorAll(CONTROLS)].filter(
    (el) => visible(el) && live(el) && !el.closest('.mc-list') && !(el instanceof HTMLInputElement && el.closest('.mc-field')),
  )
  const fields = [...stage.querySelectorAll('.mc-field')].filter((el) => visible(el) && live(el))
  const solids = [...controls.filter((el) => !el.closest('.mc-field')), ...fields, ...[...stage.querySelectorAll('.mc-list')].filter(live)]
  for (let i = 0; i < solids.length; i++) {
    for (let j = i + 1; j < solids.length; j++) {
      const a = solids[i]
      const b = solids[j]
      if (a.contains(b) || b.contains(a)) continue
      if (area(box(a), box(b)) > 1) add('controles-solapados', a, box(b))
    }
  }
  const free = liveCanvases.filter((c) => !c.closest('.mc-row'))
  for (let i = 0; i < free.length; i++) {
    for (let j = i + 1; j < free.length; j++) {
      if (free[i].closest(CONTAINERS) && free[i].closest(CONTAINERS) === free[j].closest(CONTAINERS)) continue
      if (area(box(free[i]), box(free[j])) > 1) add('textos-solapados', free[i], box(free[j]))
    }
    // Un texto suelto no puede quedar debajo de un control que no lo contiene.
    if (free[i].closest(CONTAINERS)) continue
    // (El texto de «vacia» o «cargando» de una lista se dibuja sobre ella a proposito.)
    for (const solid of solids) if (!solid.classList.contains('mc-list') && area(box(free[i]), box(solid)) > 1) add('texto-bajo-un-control', free[i], box(solid))
  }

  // 4. Medidas enteras en px de GUI.
  for (const el of stage.querySelectorAll(MEASURED)) {
    if (!visible(el) || el.closest('.splash, .mc-progress')) continue
    const r = el.getBoundingClientRect()
    const values = [(r.left - stageBox.l) / s, (r.top - stageBox.t) / s, r.width / s, r.height / s]
    if (values.some((v) => Math.abs(v - Math.round(v)) > 0.02)) add('medida-no-entera', el)
  }

  // 5. Ningun texto HTML visible.
  const walker = document.createTreeWalker(stage, NodeFilter.SHOW_TEXT)
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const parent = node.parentElement
    if (!parent || !node.textContent?.trim()) continue
    if (parent.closest('.sr-only, title, script, style')) continue
    add('texto-html-visible', parent)
  }

  // 6. Foco y 7. tamano de los controles.
  const seen = new Set<Element>()
  for (const el of stage.querySelectorAll(CONTROLS)) {
    if (!visible(el) || !live(el) || seen.has(el) || (el.closest('.mc-list') && el.getAttribute('role') !== 'listbox')) continue
    seen.add(el)
    const before = document.activeElement
    ;(el as HTMLElement).focus({ preventScroll: true })
    if (document.activeElement !== el) add('sin-foco', el)
    ;(before as HTMLElement | null)?.focus?.({ preventScroll: true })
    if (el.hasAttribute('data-audit-small') || el.closest('[data-audit-small]')) continue
    const target = el.closest('.mc-field') ?? el
    const r = target.getBoundingClientRect()
    const dpr = window.devicePixelRatio || 1
    if (r.width * dpr < 39.5 || r.height * dpr < 39.5) add('objetivo-pequeno', target)
  }
  return problems
}

window.__mcAudit = auditStage
// Solo DEV: permite cambiar de pantalla desde la consola para auditarlas todas.
window.__mcNav = nav
