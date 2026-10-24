<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from '../lib/api'
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McSlider from '../lib/mc/McSlider.svelte'
  import McText from '../lib/mc/McText.svelte'
  import McToggle from '../lib/mc/McToggle.svelte'
  import McViewport from '../lib/mc/McViewport.svelte'
  import SkinPreview3D from '../lib/mc/SkinPreview3D.svelte'
  import BitmapText from '../lib/mc/BitmapText.svelte'
  import { useGui } from '../lib/mc/gui'
  import { posStyle } from '../lib/mc/pos'
  import {
    History,
    SIZE,
    BODY_GROUPS,
    MC_COLORS,
    blankSkin,
    boxParts,
    extractPalette,
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

  const gui = useGui()

  let sub = $state<'main' | 'options' | 'setup'>(skins.editingId ? 'main' : 'setup')
  let help = $state(false)

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
  let pixelDirty = $state(false)
  let savedName = $state('Mi skin')
  let savedModel = $state<SkinModel>('classic')
  const dirty = $derived(pixelDirty || name !== savedName || model !== savedModel)
  let loading = $state(true)
  let loadFailed = $state(false)

  // ---- Herramientas --------------------------------------------------------------------------
  let tool = $state<Tool>('pencil')
  let color = $state('#c0392b')
  let layer = $state<Layer>('base')
  let group = $state<BodyGroup>('all')
  let showBase = $state(true)
  let showOverlay = $state(true)
  let mirror = $state(false)
  let grid = $state(true)

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmLeave = $state(false)
  let previewSrc = $state('')
  let skinColors = $state<string[]>([])
  let yaw = $state(-28)

  let canvas = $state<HTMLCanvasElement>()
  // Cursor del teclado sobre el lienzo (pixel) y su descripcion para lectores de pantalla.
  let cursor = $state<[number, number]>([8, 8])
  let cursorNote = $state('')
  let canvasFocused = $state(false)
  let painting = false
  let strokeBefore: Pixels | null = null
  let lastPoint: [number, number] | null = null

  const baseMask = $derived(layerMask(model, 'base'))
  const overlayMask = $derived(layerMask(model, 'overlay'))
  const activeMask = $derived(groupMask(model, layer, group))

  // El lienzo ocupa `zoom` px de GUI por texel (entero): 2 a 240 de alto, 3 con mas altura.
  const RIGHT_W = 164
  const zoom = $derived(Math.max(1, Math.min(3, Math.floor((gui.H - 108) / 64), Math.floor((gui.W - 16 - 8 - RIGHT_W) / 64))))
  const canvasSize = $derived(SIZE * zoom)
  // El buffer interno mide w*s x h*s para dibujar a resolucion nativa; `tz` = px de buffer por texel.
  const bufferSize = $derived(Math.max(SIZE, Math.round(canvasSize * gui.s)))
  // Lienzo + columna derecha forman un bloque centrado en el escenario.
  const left = $derived(-((canvasSize + 6 + RIGHT_W) >> 1))
  const rightX = $derived(left + canvasSize + 6)

  const rgb = $derived(hexToRgba(color))
  const palette = $derived([...MC_COLORS, ...skinColors.slice(0, 16)])

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
      savedName = name
      savedModel = model
    } catch (e) {
      error = (e as Error).message
      loadFailed = true
      img = blankSkin(model)
    } finally {
      loading = false
      refreshPreview()
      version++
    }
  })

  // Esc y el boton «Volver» comparten el mismo manejo de cambios sin guardar.
  $effect(() => pushBack(sub === 'setup' ? leave : sub === 'options' ? () => (sub = 'main') : back))

  /** Decodifica el PNG de la skin. Las antiguas 64x32 se convierten a 64x64. */
  function loadSkin(url: string): Promise<Pixels> {
    return new Promise((resolve, reject) => {
      const image = new Image()
      image.onload = () => {
        if (image.width !== SIZE || ![32, SIZE].includes(image.height)) {
          reject(new Error('La skin debe medir 64×64 o 64×32 píxeles.'))
          return
        }
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
    skinColors = extractPalette(img, 16)
  }

  // ---- Dibujo -------------------------------------------------------------------------------
  $effect(() => {
    void version
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
    const tz = bufferSize / SIZE
    // Bordes enteros: los texels no dejan costuras aunque `tz` no sea entero.
    const edge = (n: number) => Math.round(n * tz)
    const fillCell = (x: number, y: number) => ctx.fillRect(edge(x), edge(y), edge(x + 1) - edge(x), edge(y + 1) - edge(y))
    ctx.clearRect(0, 0, canvas.width, canvas.height)

    // Fondo de ajedrez para distinguir la transparencia.
    for (let y = 0; y < SIZE; y++) {
      for (let x = 0; x < SIZE; x++) {
        ctx.fillStyle = (x + y) % 2 === 0 ? '#3a3a3a' : '#2c2c2c'
        fillCell(x, y)
      }
    }
    for (let y = 0; y < SIZE; y++) {
      for (let x = 0; x < SIZE; x++) {
        const i = y * SIZE + x
        const visible = (baseMask[i] && showBase) || (overlayMask[i] && showOverlay)
        const [r, g, b, a] = getPixel(img, x, y)
        if (visible && a > 0) {
          ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`
          fillCell(x, y)
        }
        // Atenuar lo que no pertenece a la capa activa (guia visual).
        if (!activeMask[i]) {
          ctx.fillStyle = baseMask[i] || overlayMask[i] ? 'rgba(0,0,0,0.55)' : 'rgba(0,0,0,0.8)'
          fillCell(x, y)
        }
      }
    }
    drawPartGuides(ctx, tz)
    if (canvasFocused) {
      const lw = Math.max(1, Math.round(tz / 2))
      ctx.strokeStyle = '#ffffff'
      ctx.lineWidth = lw
      ctx.strokeRect(edge(cursor[0]) + lw / 2, edge(cursor[1]) + lw / 2, tz - lw, tz - lw)
    }
    if (grid && tz >= 4) {
      ctx.strokeStyle = 'rgba(255,255,255,0.10)'
      ctx.lineWidth = 1
      ctx.beginPath()
      for (let i = 0; i <= SIZE; i++) {
        ctx.moveTo(edge(i) + 0.5, 0)
        ctx.lineTo(edge(i) + 0.5, bufferSize)
        ctx.moveTo(0, edge(i) + 0.5)
        ctx.lineTo(bufferSize, edge(i) + 0.5)
      }
      ctx.stroke()
    }
  })

  /** Color de la guia de cada grupo. */
  const GUIDE_COLOR: Record<BodyGroup, string> = { all: '#ffffff', head: '#5aa9ff', body: '#5fd38a', arms: '#ffb04a', legs: '#c58bff' }

  /** Contornos de color de cada parte del cuerpo de la capa activa (sin texto: el escenario solo admite la fuente bitmap). */
  function drawPartGuides(ctx: CanvasRenderingContext2D, tz: number) {
    ctx.save()
    const lw = Math.max(1, Math.round(tz / 2))
    for (const part of boxParts(model).filter((p) => p.layer === layer)) {
      const g = groupOf(part.name)
      const focused = group === 'all' || group === g
      const faces = faceRects(part)
      ctx.globalAlpha = focused ? 1 : 0.35
      ctx.strokeStyle = 'rgba(255,255,255,0.28)'
      ctx.lineWidth = 1
      for (const r of Object.values(faces)) ctx.strokeRect(Math.round(r.x * tz) + 0.5, Math.round(r.y * tz) + 0.5, Math.round(r.w * tz) - 1, Math.round(r.h * tz) - 1)
      ctx.strokeStyle = GUIDE_COLOR[g]
      ctx.lineWidth = lw
      ctx.strokeRect(part.u * tz + lw / 2, part.v * tz + lw / 2, (2 * part.d + 2 * part.w) * tz - lw, (part.d + part.h) * tz - lw)
    }
    ctx.restore()
  }

  /** Al elegir una parte, el cursor del teclado salta a su cara frontal. */
  function focusGroup(next: BodyGroup) {
    group = next
    const part = boxParts(model).find((p) => p.layer === layer && groupOf(p.name) === (next === 'all' ? 'head' : next))
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
    if (loading || loadFailed) return
    if (tool === 'picker') {
      const [r, g, b, a] = getPixel(img, p[0], p[1])
      if (a > 0) color = rgbaToHex([r, g, b, a])
      tool = 'pencil'
      return
    }

    endStroke()
    const before = new Uint8ClampedArray(img)
    if (tool === 'fill') {
      const targets: [number, number][] = [p]
      if (mirror) {
        const m = mirrorPixel(model, p[0], p[1])
        if (m) targets.push(m)
      }
      let changed = 0
      // El relleno se limita a la cara pulsada: es la unidad natural de pintura de una skin.
      for (const [x, y] of targets) changed += floodFill(img, x, y, hexToRgba(color), faceMaskAt(model, layer, x, y, group))
      if (changed > 0 && history.record(before, img)) markChanged()
      return
    }

    painting = true
    strokeBefore = before
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
    if (strokeBefore && history.record(strokeBefore, img)) markChanged()
    strokeBefore = null
  }

  function markChanged() {
    historyVersion++
    version++
    pixelDirty = true
    refreshPreview()
  }

  function undo() {
    if (loading || loadFailed) return
    endStroke()
    const previous = history.undo(img)
    if (previous) {
      img = previous
      markChanged()
    }
  }

  function redo() {
    if (loading || loadFailed) return
    endStroke()
    const next = history.redo(img)
    if (next) {
      img = next
      markChanged()
    }
  }

  function onKey(e: KeyboardEvent) {
    if (help || confirmLeave || error || notice) return
    const target = e.target as HTMLElement
    if (['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName)) return
    const mod = e.ctrlKey || e.metaKey
    if (mod && e.key.toLowerCase() === 'z') {
      e.preventDefault()
      if (e.shiftKey) redo()
      else undo()
    } else if (mod && e.key.toLowerCase() === 'y') {
      e.preventDefault()
      redo()
    } else if (!mod && sub === 'main') {
      const map: Record<string, Tool> = { b: 'pencil', e: 'eraser', g: 'fill', i: 'picker' }
      const next = map[e.key.toLowerCase()]
      if (next) tool = next
    }
  }

  function setChannel(channel: 0 | 1 | 2, value: number) {
    const next: Rgba = [rgb[0], rgb[1], rgb[2], 255]
    next[channel] = Math.max(0, Math.min(255, Math.round(value)))
    color = rgbaToHex(next)
  }

  // ---- Guardar y salir ----------------------------------------------------------------------
  async function save(): Promise<boolean> {
    if (saving || loading || loadFailed) return false
    endStroke()
    const saveVersion = version
    const saveName = name
    const saveModel = model
    saving = true
    error = null
    notice = null
    try {
      const saved = await api.skinsSave(skinId, saveName, previewSrc, saveModel)
      skinId = saved.id
      skins.editingId = saved.id
      if (version === saveVersion) pixelDirty = false
      savedName = saveName
      savedModel = saveModel
      skins.invalidatePreview(saved.id)
      await skins.load(true)
      notice = 'Guardada. Vuelve a Skins y pulsa Usar.'
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
    endStroke()
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
  const layers = [
    { value: 'base' as Layer, label: 'Base' },
    { value: 'overlay' as Layer, label: 'Exterior' },
  ]
  const models = [
    { value: 'classic' as SkinModel, label: 'Clásico' },
    { value: 'slim' as SkinModel, label: 'Fino' },
  ]
  const groups = BODY_GROUPS.map((g) => ({ value: g.id, label: g.label }))
</script>

<svelte:window onkeydown={onKey} onpointerup={endStroke} onpointercancel={endStroke} />

{#if sub === 'setup'}
  <McScreen title="Crear skin · 1 de 2">
    <McLabel x={-150} y={30} text="1. Ponle un nombre" />
    <McField x={-150} y={44} w={300} maxlength={48} bind:value={name} label="Nombre de la skin" />
    <McCycle x={-150} y={76} w={300} label="Brazos" options={models} bind:value={model} />
    <McText x={-150} y={104} w={300} text="Clásico: brazos anchos. Fino: brazos como Alex. Empezarás con una base coloreada lista para pintar." maxLines={3} />
    <McText x={-150} y={144} w={300} text="2. Elige una parte, un color y pinta. Revisa el resultado en 3D; puedes deshacer cada trazo." maxLines={3} />
    <McButton x={-150} w={146} anchor="bottom" y={-28} label="Cancelar" onclick={leave} />
    <McButton x={4} w={146} anchor="bottom" y={-28} label="Empezar a pintar" disabled={loading || !name.trim()} onclick={() => {
      img = blankSkin(model); savedModel = model; savedName = name; refreshPreview(); version++; sub = 'main'
    }} />
  </McScreen>
{:else if sub === 'options'}
  <McScreen title="Opciones del editor">
    <McCycle x={-155} y={24} w={150} label="Capa" options={layers} value={layer} onchange={(next) => { layer = next; focusGroup(group) }} />
    <McCycle x={5} y={24} w={150} label="Parte" options={groups} value={group} onchange={focusGroup} />
    <McToggle x={-155} y={48} w={150} label="Ver capa base" bind:checked={showBase} />
    <McToggle x={5} y={48} w={150} label="Ver capa exterior" bind:checked={showOverlay} />
    <McToggle x={-155} y={72} w={150} label="Espejo" bind:checked={mirror} />
    <McToggle x={5} y={72} w={150} label="Cuadrícula" bind:checked={grid} />
    <McCycle x={-155} y={96} w={150} label="Modelo" options={models} value={model} onchange={(next) => { model = next; focusGroup(group) }} />
    <McLabel x={-155} y={124} color="#a0a0a0" text="Nombre" />
    <McField x={-155} y={134} w={310} maxlength={48} bind:value={name} placeholder="Mi skin" label="Nombre de la skin" />
    <McButton x={-150} w={146} anchor="bottom" y={-28} label="Cómo pintar..." onclick={() => { sub = 'main'; help = true }} />
    <McButton x={4} w={146} anchor="bottom" y={-28} label="Hecho" onclick={() => (sub = 'main')} />
  </McScreen>
{:else}
  <McScreen title={`${skinId ? 'Editar skin' : 'Crear skin · 2 de 2'}${dirty ? ' *' : ''}`}>
    <form
      class="mc-form"
      onsubmit={(e) => {
        e.preventDefault()
        save()
      }}
    >
      <McViewport x={left} y={22} w={canvasSize} h={canvasSize} frame={false}>
        <canvas
          bind:this={canvas}
          class="skin-canvas"
          width={bufferSize}
          height={bufferSize}
          style="width:{canvasSize}px;height:{canvasSize}px;cursor:{tool === 'picker' ? 'copy' : 'crosshair'}"
          tabindex="0"
          aria-label="Lienzo de la skin de 64 por 64 píxeles. Se dibuja con el ratón o un lápiz táctil; con el teclado, flechas para moverte (Mayús salta 8 píxeles) y Intro o Espacio para pintar."
          onkeydown={onCanvasKey}
          onfocus={() => (canvasFocused = true)}
          onblur={() => (canvasFocused = false)}
          onpointerdown={onPointerDown}
          onpointermove={onPointerMove}
          onlostpointercapture={endStroke}
        ></canvas>
      </McViewport>
      <p class="sr-only" role="status" aria-live="polite">{cursorNote}</p>

      <McViewport x={rightX} y={22} w={80} h={100}>
        <SkinPreview3D src={previewSrc} {model} height={64} unit={2} bind:yaw />
      </McViewport>
      {#each tools as t, i (t.id)}
        <McButton
          x={rightX + 84}
          y={22 + i * 24}
          w={80}
          label={t.label}
          ariaLabel="{t.label} ({t.key})"
          active={tool === t.id}
          onclick={() => (tool = t.id)}
          extra={{ 'aria-pressed': tool === t.id }}
        />
      {/each}

      <div data-audit-small>
        {#each palette as swatch, i (i)}
          <button
            type="button"
            class="mc-swatch"
            class:selected={swatch === color}
            data-audit-small
            style="{posStyle(rightX + (i % 8) * 10, 124 + Math.floor(i / 8) * 10)};background:{swatch}"
            aria-label="Color {swatch}"
            aria-pressed={swatch === color}
            onclick={() => (color = swatch)}
          ></button>
        {/each}
      </div>
      <div class="mc-sample" style="{posStyle(rightX, 168)};width:10px;height:10px;background:{color}" aria-hidden="true"></div>
      <div class="mc-label" style={posStyle(rightX + 14, 169)}><BitmapText text={color.toUpperCase()} sr={false} /></div>
      <McSlider x={rightX + 84} y={124} w={80} value={rgb[0]} min={0} max={255} format={(v) => `R: ${v}`} label="Rojo" onchange={(v) => setChannel(0, v)} />
      <McSlider x={rightX + 84} y={144} w={80} value={rgb[1]} min={0} max={255} format={(v) => `G: ${v}`} label="Verde" onchange={(v) => setChannel(1, v)} />
      <McSlider x={rightX + 84} y={164} w={80} value={rgb[2]} min={0} max={255} format={(v) => `B: ${v}`} label="Azul" onchange={(v) => setChannel(2, v)} />

      <McCycle x={left} y={26 + canvasSize} w={canvasSize} label="Parte" options={groups} value={group} onchange={focusGroup} />
      {#if loading}
        <McText text="Cargando..." x={left} y={22 + (canvasSize >> 1) - 5} w={canvasSize} align="center" color="#a0a0a0" />
      {/if}
      <McButton anchor="bottom" x={-155} y={-52} w={100} label="Deshacer" disabled={!canUndo} onclick={undo} />
      <McButton anchor="bottom" x={-50} y={-52} w={100} label="Rehacer" disabled={!canRedo} onclick={redo} />
      <McButton anchor="bottom" x={55} y={-52} w={100} label="Opciones / ayuda" ariaLabel="Opciones del editor" onclick={() => (sub = 'options')} />
      <McButton anchor="bottom" x={-155} y={-28} w={100} type="submit" label={saving ? 'Guardando…' : 'Guardar'} disabled={saving || loading || loadFailed || name.trim().length === 0} />
      <McButton anchor="bottom" x={-50} y={-28} w={100} label="Exportar PNG..." ariaLabel="Guardar y exportar PNG" disabled={saving || loading || loadFailed || name.trim().length === 0} onclick={saveAndExport} />
      <McButton anchor="bottom" x={55} y={-28} w={100} label="Volver" onclick={back} />
    </form>

    {#if help}
      <McDialog title="Pinta tu skin" message="Elige una parte para resaltar su zona en el lienzo. Selecciona un color y usa Lápiz o Relleno. Arrastra el personaje para girarlo. Deshacer recupera el último trazo. En Opciones puedes cambiar la capa, activar espejo y renombrar. Guarda y vuelve a Skins para pulsar Usar en tu perfil." scroll buttons={[{ label: 'Entendido', onclick: () => (help = false) }]} onclose={() => (help = false)} />
    {/if}
    {#if confirmLeave}
      <McDialog
        title="¿Salir sin guardar?"
        message="Tienes cambios sin guardar. Si sales ahora se perderán."
        buttons={[
          { label: 'Seguir editando', onclick: () => (confirmLeave = false) },
          { label: 'Salir sin guardar', onclick: leave },
        ]}
        cancelIndex={0}
        onclose={() => (confirmLeave = false)}
      />
    {/if}
  </McScreen>
{/if}

{#if error || notice}
  <McDialog title={error ? 'Error de skin' : 'Skin guardada'} message={error ?? notice ?? ''} error={!!error} buttons={[{ label: 'Hecho', onclick: () => { error = null; notice = null } }]} onclose={() => { error = null; notice = null }} />
{/if}

<style>
  .skin-canvas {
    display: block;
    image-rendering: pixelated;
    outline-offset: 0;
  }
</style>
