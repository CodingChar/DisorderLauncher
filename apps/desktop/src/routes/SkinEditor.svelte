<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from '../lib/api'
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import SkinPreview3D from '../lib/components/SkinPreview3D.svelte'
  import {
    History,
    SIZE,
    blankSkin,
    faceMaskAt,
    floodFill,
    getPixel,
    hexToRgba,
    layerMask,
    linePoints,
    mirrorPixel,
    rgbaToHex,
    setPixel,
    upgradeLegacy,
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
  let showBase = $state(true)
  let showOverlay = $state(true)
  let mirror = $state(false)
  let grid = $state(true)
  let zoom = $state(7)

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmLeave = $state(false)
  let previewSrc = $state('')

  let canvas = $state<HTMLCanvasElement>()
  let painting = false
  let lastPoint: [number, number] | null = null

  const baseMask = $derived(layerMask(model, 'base'))
  const overlayMask = $derived(layerMask(model, 'overlay'))
  const activeMask = $derived(layer === 'base' ? baseMask : overlayMask)

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
    void model
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
  <header>
    <small>EDITOR DE SKINS</small>
    <h1>{skinId ? 'Editar skin' : 'Nueva skin'}</h1>
  </header>

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

      <div class="canvas-wrap" style:--zoom="{zoom}">
        <canvas
          bind:this={canvas}
          width={SIZE * zoom}
          height={SIZE * zoom}
          aria-label="Lienzo de la skin de 64 por 64 píxeles. Se dibuja con el ratón o un lápiz táctil."
          onpointerdown={onPointerDown}
          onpointermove={onPointerMove}
          style:cursor={tool === 'picker' ? 'copy' : 'crosshair'}
        ></canvas>
      </div>

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

  header small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.18em;
    margin-bottom: 5px;
  }

  h1 {
    font-size: 34px;
    line-height: 0.95;
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
