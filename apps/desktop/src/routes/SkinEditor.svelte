<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from '../lib/api'
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import SkinPreview3D from '../lib/components/SkinPreview3D.svelte'
  import {
    History,
    SIZE,
    BODY_GROUPS,
    PART_LABEL,
    blankSkin,
    boxParts,
    faceMaskAt,
    faceRects,
    groupMask,
    groupOf,
    floodFill,
    getPixel,
    hexToRgba,
    layerMask,
    linePoints,
    mirrorPixel,
    rgbaToHex,
    setPixel,
    upgradeLegacy,
    type BodyGroup,
    type Layer,
    type Pixels,
    type Rgba,
  } from '../lib/skinEditor'
  import { nav } from '../lib/stores/nav.svelte'
  import { skins } from '../lib/stores/skins.svelte'
  import type { SkinModel } from '../lib/types'

  type Tool = 'pencil' | 'eraser' | 'fill' | 'picker'

  // ---- Estado del documento ----------------------------------------------------------------
  let skinId = $state<string | null>(skins.editingId)
  let name = $state('Mi skin')
  let model = $state<SkinModel>('classic')
  let img: Pixels = new Uint8ClampedArray(SIZE * SIZE * 4)
  const history = new History()
  // `History` no es reactivo: este contador avisa a la interfaz cuando cambia.
  let historyVersion = $state(0)
  const canUndo = $derived.by(() => {
    void historyVersion
    return history.canUndo
  })
  const canRedo = $derived.by(() => {
    void historyVersion
    return history.canRedo
  })
  let version = $state(0) // sube con cada cambio de pixeles para redibujar
  let dirty = $state(false)
  let loading = $state(true)

  // ---- Herramientas --------------------------------------------------------------------------
  let tool = $state<Tool>('pencil')
  let color = $state('#c0392b')
  let layer = $state<Layer>('base')
  let group = $state<BodyGroup>('all')
  let showBase = $state(true)
  let showOverlay = $state(true)
  let mirror = $state(false)
  let grid = $state(true)
  let zoom = $state(9)

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmLeave = $state(false)
  let previewSrc = $state('')

  let canvas = $state<HTMLCanvasElement>()
  // Cursor del teclado sobre el lienzo (pixel) y su descripcion para lectores de pantalla.
  let cursor = $state<[number, number]>([8, 8])
  let cursorNote = $state('')
  let canvasFocused = $state(false)
  let painting = false
  let lastPoint: [number, number] | null = null

  const baseMask = $derived(layerMask(model, 'base'))
  const overlayMask = $derived(layerMask(model, 'overlay'))
  const activeMask = $derived(groupMask(model, layer, group))

  onMount(async () => {
    try {
      if (skinId) {
        const asset = skins.list.find((s) => s.id === skinId)
        name = asset?.name ?? name
        model = asset?.model ?? model
        img = await loadSkin(await api.skinsRead(skinId))
      } else {
        img = blankSkin(model)
      }
    } catch (e) {
      error = (e as Error).message
      img = blankSkin(model)
    } finally {
      loading = false
      refreshPreview()
      version++
    }
  })

  /** Decodifica el PNG de la skin. Las antiguas 64x32 se convierten a 64x64. */
  function loadSkin(url: string): Promise<Pixels> {
    return new Promise((resolve, reject) => {
      const image = new Image()
      image.onload = () => {
        const c = document.createElement('canvas')
        c.width = image.width
        c.height = image.height
        const ctx = c.getContext('2d')!
        ctx.drawImage(image, 0, 0)
        const data = ctx.getImageData(0, 0, image.width, image.height).data
        resolve(image.height === 32 ? upgradeLegacy(data) : new Uint8ClampedArray(data))
      }
      image.onerror = () => reject(new Error('No se pudo abrir la skin.'))
      image.src = url
    })
  }

  function toPngDataUrl(): string {
    const c = document.createElement('canvas')
    c.width = SIZE
    c.height = SIZE
    c.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(img), SIZE, SIZE), 0, 0)
    return c.toDataURL('image/png')
  }

  function refreshPreview() {
    previewSrc = toPngDataUrl()
  }

  // ---- Dibujo -------------------------------------------------------------------------------
  $effect(() => {
    void version
    void zoom
    void grid
    void showBase
    void showOverlay
    void layer
    void group
    void model
    void cursor
    void canvasFocused
    if (!canvas) return
    const ctx = canvas.getContext('2d')!
    const z = zoom
    ctx.clearRect(0, 0, canvas.width, canvas.height)

    // Fondo de ajedrez para distinguir la transparencia.
    for (let y = 0; y < SIZE; y++) {
      for (let x = 0; x < SIZE; x++) {
        ctx.fillStyle = (x + y) % 2 === 0 ? '#3a3a3a' : '#2c2c2c'
        ctx.fillRect(x * z, y * z, z, z)
      }
    }
    for (let y = 0; y < SIZE; y++) {
      for (let x = 0; x < SIZE; x++) {
        const i = y * SIZE + x
        const visible = (baseMask[i] && showBase) || (overlayMask[i] && showOverlay)
        const [r, g, b, a] = getPixel(img, x, y)
        if (visible && a > 0) {
          ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`
          ctx.fillRect(x * z, y * z, z, z)
        }
        // Atenuar lo que no pertenece a la capa activa (guia visual).
        if (!activeMask[i]) {
          ctx.fillStyle = baseMask[i] || overlayMask[i] ? 'rgba(0,0,0,0.55)' : 'rgba(0,0,0,0.8)'
          ctx.fillRect(x * z, y * z, z, z)
        }
      }
    }
    drawPartGuides(ctx, z)
    if (canvasFocused) {
      ctx.strokeStyle = '#ffffff'
      ctx.lineWidth = 2
      ctx.strokeRect(cursor[0] * z + 1, cursor[1] * z + 1, z - 2, z - 2)
      ctx.strokeStyle = '#000000'
      ctx.lineWidth = 1
      ctx.strokeRect(cursor[0] * z + 3, cursor[1] * z + 3, z - 6, z - 6)
    }
    if (grid && z >= 8) {
      ctx.strokeStyle = 'rgba(255,255,255,0.08)'
      ctx.lineWidth = 1
      ctx.beginPath()
      for (let i = 0; i <= SIZE; i++) {
        ctx.moveTo(i * z + 0.5, 0)
        ctx.lineTo(i * z + 0.5, SIZE * z)
        ctx.moveTo(0, i * z + 0.5)
        ctx.lineTo(SIZE * z, i * z + 0.5)
      }
      ctx.stroke()
    }
  })

  /** Color de la guia de cada grupo; se repite en la leyenda de debajo del lienzo. */
  const GUIDE_COLOR: Record<BodyGroup, string> = { all: '#ffffff', head: '#5aa9ff', body: '#5fd38a', arms: '#ffb04a', legs: '#c58bff' }

  /** Recuadros con el nombre de cada parte del cuerpo de la capa activa, para saber que se pinta. */
  function drawPartGuides(ctx: CanvasRenderingContext2D, z: number) {
    ctx.save()
    for (const part of boxParts(model).filter((p) => p.layer === layer)) {
      const g = groupOf(part.name)
      const focused = group === 'all' || group === g
      const faces = faceRects(part)
      ctx.globalAlpha = focused ? 1 : 0.35
      // Division entre caras.
      ctx.strokeStyle = 'rgba(255,255,255,0.28)'
      ctx.lineWidth = 1
      for (const r of Object.values(faces)) ctx.strokeRect(r.x * z + 0.5, r.y * z + 0.5, r.w * z - 1, r.h * z - 1)
      // Contorno de la pieza completa, del color de su grupo.
      const x0 = part.u * z
      const y0 = part.v * z
      const w = (2 * part.d + 2 * part.w) * z
      const h = (part.d + part.h) * z
      ctx.strokeStyle = GUIDE_COLOR[g]
      ctx.lineWidth = 2
      ctx.beginPath()
      ctx.rect(x0 + 1, y0 + 1, w - 2, h - 2)
      ctx.stroke()
      // Nombre sobre la cara frontal (en dos lineas si no cabe) y «frente» para orientarse.
      const front = faces.front
      const label = PART_LABEL[part.name] ?? part.name
      const lines = label.includes(' ') ? label.split(' ') : [label]
      const fontPx = Math.max(8, Math.min(12, Math.round(z * 1.3)))
      ctx.font = `700 ${fontPx}px system-ui, sans-serif`
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      const cx = (front.x + front.w / 2) * z
      const cy = (front.y + front.h / 2) * z
      lines.forEach((line, i) => {
        const y = cy + (i - (lines.length - 1) / 2) * (fontPx + 1)
        ctx.lineWidth = 3
        ctx.strokeStyle = 'rgba(0,0,0,0.85)'
        ctx.strokeText(line, cx, y)
        ctx.fillStyle = GUIDE_COLOR[g]
        ctx.fillText(line, cx, y)
      })
    }
    ctx.restore()
  }

  /** Al elegir una parte, el cursor del teclado salta a su cara frontal. */
  function focusGroup(next: BodyGroup) {
    group = next
    if (next === 'all') return
    const part = boxParts(model).find((p) => p.layer === layer && groupOf(p.name) === next)
    if (part) {
      const f = faceRects(part).front
      cursor = [f.x + Math.floor(f.w / 2), f.y + Math.floor(f.h / 2)]
    }
  }

  function pixelAt(e: PointerEvent): [number, number] | null {
    if (!canvas) return null
    const rect = canvas.getBoundingClientRect()
    const x = Math.floor(((e.clientX - rect.left) / rect.width) * SIZE)
    const y = Math.floor(((e.clientY - rect.top) / rect.height) * SIZE)
    return x >= 0 && y >= 0 && x < SIZE && y < SIZE ? [x, y] : null
  }

  function paint(x: number, y: number, rgba: Rgba) {
    const targets: [number, number][] = [[x, y]]
    if (mirror) {
      const m = mirrorPixel(model, x, y)
      if (m) targets.push(m)
    }
    for (const [tx, ty] of targets) {
      if (activeMask[ty * SIZE + tx]) setPixel(img, tx, ty, rgba)
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0 || loading) return
    const p = pixelAt(e)
    if (!p) return
    e.preventDefault()
    canvas?.setPointerCapture(e.pointerId)
    beginAt(p)
  }

  /** Aplica la herramienta activa en un pixel; el trazo de lapiz/borrador queda abierto hasta `endStroke`. */
  function beginAt(p: [number, number]) {
    if (tool === 'picker') {
      const [r, g, b, a] = getPixel(img, p[0], p[1])
      if (a > 0) color = rgbaToHex([r, g, b, a])
      tool = 'pencil'
      return
    }

    history.push(img)
    historyVersion++
    if (tool === 'fill') {
      const targets: [number, number][] = [p]
      if (mirror) {
        const m = mirrorPixel(model, p[0], p[1])
        if (m) targets.push(m)
      }
      let changed = 0
      // El relleno se limita a la cara pulsada: es la unidad natural de pintura de una skin.
      for (const [x, y] of targets) changed += floodFill(img, x, y, hexToRgba(color), faceMaskAt(model, layer, x, y))
      if (changed === 0) {
        history.undo(img)
        historyVersion++
      } else markChanged()
      return
    }

    painting = true
    lastPoint = null
    stroke(p)
  }

  // Alternativa de teclado: flechas mueven el cursor (Mayus = 8 pixeles) y Enter o Espacio pintan.
  function onCanvasKey(e: KeyboardEvent) {
    if (loading || e.ctrlKey || e.metaKey || e.altKey) return
    const step = e.shiftKey ? 8 : 1
    const move: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }
    const delta = move[e.key]
    if (delta) {
      e.preventDefault()
      cursor = [Math.max(0, Math.min(SIZE - 1, cursor[0] + delta[0])), Math.max(0, Math.min(SIZE - 1, cursor[1] + delta[1]))]
      cursorNote = `Pixel ${cursor[0] + 1}, ${cursor[1] + 1}`
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      beginAt(cursor)
      endStroke()
    }
  }

  function stroke(p: [number, number]) {
    const rgba: Rgba = tool === 'eraser' ? [0, 0, 0, 0] : hexToRgba(color)
    const points = lastPoint ? linePoints(lastPoint[0], lastPoint[1], p[0], p[1]) : [p]
    for (const [x, y] of points) paint(x, y, rgba)
    lastPoint = p
    version++
    dirty = true
  }

  function onPointerMove(e: PointerEvent) {
    if (!painting) return
    const p = pixelAt(e)
    if (p && (!lastPoint || p[0] !== lastPoint[0] || p[1] !== lastPoint[1])) stroke(p)
  }

  function endStroke() {
    if (!painting) return
    painting = false
    lastPoint = null
    refreshPreview()
  }

  function markChanged() {
    historyVersion++
    version++
    dirty = true
    refreshPreview()
  }

  function undo() {
    const previous = history.undo(img)
    if (previous) {
      img = previous
      markChanged()
    }
  }

  function redo() {
    const next = history.redo(img)
    if (next) {
      img = next
      markChanged()
    }
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement
    if (['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName)) return
    const mod = e.ctrlKey || e.metaKey
    if (mod && e.key.toLowerCase() === 'z') {
      e.preventDefault()
      e.shiftKey ? redo() : undo()
    } else if (mod && e.key.toLowerCase() === 'y') {
      e.preventDefault()
      redo()
    } else if (!mod) {
      const map: Record<string, Tool> = { b: 'pencil', e: 'eraser', g: 'fill', i: 'picker' }
      const next = map[e.key.toLowerCase()]
      if (next) tool = next
    }
  }

  // ---- Guardar y salir ----------------------------------------------------------------------
  async function save(): Promise<boolean> {
    if (saving) return false
    saving = true
    error = null
    notice = null
    try {
      const saved = await api.skinsSave(skinId, name, toPngDataUrl(), model)
      skinId = saved.id
      skins.editingId = saved.id
      dirty = false
      await skins.load()
      notice = 'Skin guardada en tu biblioteca.'
      return true
    } catch (e) {
      error = (e as Error).message
      return false
    } finally {
      saving = false
    }
  }

  async function saveAndExport() {
    if (await save()) {
      try {
        await skins.exportSkin(skinId!)
      } catch (e) {
        error = (e as Error).message
      }
    }
  }

  function back() {
    if (dirty) confirmLeave = true
    else leave()
  }

  function leave() {
    confirmLeave = false
    skins.editingId = null
    nav.go('skins')
  }

  const tools: { id: Tool; label: string; key: string }[] = [
    { id: 'pencil', label: 'Lápiz', key: 'B' },
    { id: 'eraser', label: 'Borrador', key: 'E' },
    { id: 'fill', label: 'Relleno', key: 'G' },
    { id: 'picker', label: 'Cuentagotas', key: 'I' },
  ]
</script>

<svelte:window onkeydown={onKey} onpointerup={endStroke} />

<div class="page">
  <PageHeading eyebrow="EDITOR DE SKINS" title={skinId ? 'Editar skin' : 'Nueva skin'} />

  <div class="layout">
    <Card>
      <div class="tools" role="toolbar" aria-label="Herramientas">
        {#each tools as t (t.id)}
          <button type="button" class:active={tool === t.id} aria-pressed={tool === t.id} onclick={() => (tool = t.id)} title="{t.label} ({t.key})">
            {t.label}
          </button>
        {/each}
        <label class="color">
          <span>Color</span>
          <input type="color" bind:value={color} aria-label="Color de dibujo" />
        </label>
      </div>

      <div class="options">
        <fieldset>
          <legend>Parte del cuerpo</legend>
          {#each BODY_GROUPS as g (g.id)}
            <label><input type="radio" name="body-group" checked={group === g.id} onchange={() => focusGroup(g.id)} /> {g.label}</label>
          {/each}
        </fieldset>
        <fieldset>
          <legend>Capa</legend>
          <label><input type="radio" bind:group={layer} value="base" /> Base</label>
          <label><input type="radio" bind:group={layer} value="overlay" /> Exterior (sombrero, chaqueta…)</label>
        </fieldset>
        <fieldset>
          <legend>Mostrar</legend>
          <label><input type="checkbox" bind:checked={showBase} /> Capa base</label>
          <label><input type="checkbox" bind:checked={showOverlay} /> Capa exterior</label>
        </fieldset>
        <fieldset>
          <legend>Ayudas</legend>
          <label><input type="checkbox" bind:checked={mirror} /> Espejo (simetría)</label>
          <label><input type="checkbox" bind:checked={grid} /> Cuadrícula</label>
        </fieldset>
      </div>

      <p class="guide-help">
        Cada recuadro de color es una parte del cuerpo y dentro está dibujada su cara frontal. Elige una <strong>parte del cuerpo</strong> para trabajar solo en ella: lo demás se oscurece y no se pinta.
      </p>
      <ul class="legend" aria-label="Colores de las partes">
        {#each BODY_GROUPS.filter((g) => g.id !== 'all') as g (g.id)}
          <li><span class="chip" style:background={GUIDE_COLOR[g.id]}></span>{g.label}</li>
        {/each}
      </ul>

      <div class="canvas-wrap" style:--zoom="{zoom}">
        <canvas
          bind:this={canvas}
          width={SIZE * zoom}
          height={SIZE * zoom}
          tabindex="0"
          aria-label="Lienzo de la skin de 64 por 64 píxeles. Se dibuja con el ratón o un lápiz táctil; con el teclado, flechas para moverte (Mayús salta 8 píxeles) y Intro o Espacio para pintar."
          onkeydown={onCanvasKey}
          onfocus={() => (canvasFocused = true)}
          onblur={() => (canvasFocused = false)}
          onpointerdown={onPointerDown}
          onpointermove={onPointerMove}
          style:cursor={tool === 'picker' ? 'copy' : 'crosshair'}
        ></canvas>
      </div>
      <p class="sr-only" role="status" aria-live="polite">{cursorNote}</p>

      <div class="bar">
        <Button disabled={!canUndo} onclick={undo}>Deshacer</Button>
        <Button disabled={!canRedo} onclick={redo}>Rehacer</Button>
        <label class="zoom">
          Zoom
          <input type="range" min="4" max="14" step="1" bind:value={zoom} aria-label="Zoom del lienzo" />
        </label>
      </div>
    </Card>

    <div class="side">
      <Card>
        <h3>Vista 3D</h3>
        <SkinPreview3D src={previewSrc} {model} />
      </Card>

      <Card>
        <form
          onsubmit={(e) => {
            e.preventDefault()
            save()
          }}
        >
          <Input label="Nombre" bind:value={name} maxlength={48} />
          <fieldset class="model">
            <legend>Modelo de brazos</legend>
            <label><input type="radio" bind:group={model} value="classic" /> Clásico (Steve)</label>
            <label><input type="radio" bind:group={model} value="slim" /> Fino (Alex)</label>
          </fieldset>

          {#if error}<p class="error" role="alert">{error}</p>{/if}
          {#if notice}<p class="notice" role="status">{notice}</p>{/if}

          <div class="actions">
            <Button variant="primary" type="submit" disabled={saving || name.trim().length === 0 || loading}>
              {saving ? 'Guardando…' : 'Guardar'}
            </Button>
            <Button disabled={saving || loading || name.trim().length === 0} onclick={saveAndExport}>Guardar y exportar PNG…</Button>
            <Button variant="ghost" onclick={back}>Volver</Button>
          </div>
        </form>
      </Card>
    </div>
  </div>

  <ConfirmDialog
    open={confirmLeave}
    title="¿Salir sin guardar?"
    message="Tienes cambios sin guardar. Si sales ahora se perderán."
    confirmLabel="Salir sin guardar"
    cancelLabel="Seguir editando"
    onconfirm={leave}
    oncancel={() => (confirmLeave = false)}
  />
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }



  h3 {
    font-size: 18px;
    margin-bottom: 10px;
  }

  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: 16px;
    align-items: start;
  }

  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    margin-bottom: 12px;
  }

  .tools button {
    min-height: 44px;
    padding: 0 14px;
    border-radius: var(--r-md);
    border: 1px solid var(--border-strong);
    background: var(--surface-alt);
    color: var(--text-primary);
    font-weight: 600;
  }

  .tools button.active {
    background: var(--action);
    color: var(--action-text);
    border-color: var(--action);
  }

  .color {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-left: 8px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .color input {
    width: 48px;
    height: 44px;
    padding: 2px;
    border: 1px solid var(--border-strong);
    border-radius: var(--r-sm);
    background: var(--surface-alt);
  }

  .guide-help {
    margin: 0 0 8px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin: 0 0 10px;
    padding: 0;
    list-style: none;
    font-size: 12px;
  }

  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .chip {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    border: 1px solid var(--border-strong);
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 12px;
  }

  fieldset {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 6px 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  legend {
    padding: 0 6px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  fieldset label {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    font-size: 13px;
  }

  fieldset input {
    width: 18px;
    height: 18px;
    accent-color: var(--action);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .canvas-wrap {
    overflow: auto;
    max-width: 100%;
    border-radius: var(--r-sm);
    border: 1px solid var(--border);
    background: #1e1e1e;
  }

  canvas {
    display: block;
    image-rendering: pixelated;
    touch-action: none;
  }

  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    margin-top: 12px;
  }

  .zoom {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .zoom input {
    width: 140px;
    height: 32px;
    accent-color: var(--action);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .error {
    margin: 0;
    font-weight: 600;
  }

  .notice {
    margin: 0;
    color: var(--text-secondary);
  }
</style>
