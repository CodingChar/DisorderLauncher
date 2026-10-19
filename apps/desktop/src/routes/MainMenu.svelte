<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import McButton from '../lib/mc/McButton.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import BitmapText from '../lib/mc/BitmapText.svelte'
  import { renderText, stringWidth } from '../lib/mc/font'
  import { fontStatus } from '../lib/mc/fontStatus.svelte'
  import { useGui } from '../lib/mc/gui'
  import { drawLogo } from '../lib/mc/logo'
  import { animateVisible } from '../lib/mc/animation'
  import { SPLASH_ANGLE, SPLASH_COLOR, splashLayout, splashScale } from '../lib/mc/splash'
  import { posStyle } from '../lib/mc/pos'
  import { nav } from '../lib/stores/nav.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McViewport from '../lib/mc/McViewport.svelte'
  import SkinPreview3D from '../lib/mc/SkinPreview3D.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { skins } from '../lib/stores/skins.svelte'
  import { blankSkin } from '../lib/skinEditor'
  import { needsTour, finishTour, TOUR } from '../lib/onboarding'
  import { APP_VERSION } from '../lib/version'

  interface Props {
    online: boolean
    /** Permite repetir el paseo desde Ayuda. */
    tourAllowed?: boolean
    tourRequested?: boolean
    ontourend: () => void
    onhelp: () => void
  }

  let { online, tourAllowed = true, tourRequested = false, ontourend, onhelp }: Props = $props()

  const gui = useGui()
  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

  let logo = $state<HTMLCanvasElement>()
  let splashEl = $state<HTMLCanvasElement>()
  let logoWidth = $state(288)
  let logoHeight = $state(28)
  let splash = $state('¡Bienvenido!')

  const k = $derived((gui.H >> 2) + 48)
  let tour = $state(false)
  let step = $state(0)
  let fallback = $state('')
  // Botones + skin forman un solo grupo centrado respecto al logo (x=0 es el centro); se adapta al ancho de GUI.
  const menuW = $derived(Math.max(60, Math.min(200, gui.W - 128)))
  const skinW = $derived(Math.max(40, Math.min(100, gui.W - 16 - menuW - 12)))
  const menuX = $derived(-((menuW + 12 + skinW) >> 1))
  const skinX = $derived(menuX + menuW + 12)
  const account = $derived(accounts.current ?? accounts.list[0])
  const skinId = $derived(account?.providerData.type === 'local' ? account.providerData.skinId : null)
  const asset = $derived(skins.list.find((s) => s.id === skinId))
  const skinSrc = $derived((skinId && skins.previews[skinId]) || fallback)
  $effect(() => {
    if (skinId && skins.loaded) untrack(() => { void skins.ensurePreview(skinId) })
  })
  $effect(() => { if (tourRequested) { step = 0; tour = true } })
  function closeTour() { tour = false; finishTour(); ontourend() }
  const highlight = $derived(step === 4
    ? { x: skinX, y: k, w: skinW, h: 116 }
    : { x: menuX, y: k + (step === 5 ? 96 : step * 24), w: step === 5 ? Math.floor(menuW / 2) - 2 : menuW, h: 20 })

  // Y del primer boton: height / 4 + 48 (GuiMainMenu).
  const logoTop = $derived(30 + Math.round((44 - logoHeight) / 2))
  const splashWidth = $derived(stringWidth(splash))
  const splashBox = $derived(splashLayout(splashWidth, gui.cx + logoWidth / 2 - 47, logoTop + logoHeight - 4, gui.W))
  const splashGlyph = $derived(fontStatus.ready ? renderText(splash, SPLASH_COLOR) : null)
  const notes = $derived(
    [!online ? { text: 'Sin conexión: puedes jugar lo que ya descargaste', color: '#a0a0a0' } : null].filter(
      (n): n is { text: string; color: string } => n !== null,
    ),
  )

  // El logo se redibuja a resolucion nativa cuando cambia la ventana.
  $effect(() => {
    if (!logo) return
    const size = drawLogo(logo, { width: gui.W, scale: gui.factor })
    logoWidth = size.width
    logoHeight = size.height
  })

  // Como GuiMainMenu: amarillo puro, girado -20 grados y con latido de escala. Se dibuja a la resolucion
  // real del dispositivo con vecino mas cercano (como la fuente de Minecraft), sin escalar texto en CSS.
  function placeSplash(now: number, still: boolean) {
    const canvas = splashEl
    const glyph = splashGlyph
    if (!canvas || !glyph) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const scale = splashScale(now, splashWidth, still)
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.clearRect(0, 0, canvas.width, canvas.height)
    ctx.imageSmoothingEnabled = false
    // Unidades de GUI a pixeles de dispositivo; luego posicion, giro y escala del splash.
    ctx.setTransform(gui.factor, 0, 0, gui.factor, 0, 0)
    ctx.translate(splashBox.anchorX - splashBox.x, splashBox.anchorY - splashBox.y)
    ctx.rotate(SPLASH_ANGLE)
    ctx.scale(scale, scale)
    ctx.drawImage(glyph, -(splashWidth >> 1), -8)
  }

  // Buffer pequeno y fijo durante el latido; nunca se borra toda la ventana para animar una frase.
  $effect(() => {
    const canvas = splashEl
    if (!canvas) return
    canvas.width = splashBox.width * gui.factor
    canvas.height = splashBox.height * gui.factor
    placeSplash(performance.now(), matchMedia('(prefers-reduced-motion: reduce)').matches)
  })

  onMount(() => {
    tour = needsTour()
    void skins.load()
    const c = document.createElement('canvas')
    c.width = c.height = 64
    c.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(blankSkin('classic')), 64, 64), 0, 0)
    fallback = c.toDataURL('image/png')
    const stopAnimation = animateVisible(placeSplash)
    let alive = true
    const controller = new AbortController()
    fetch('/assets/splashes.txt', { signal: controller.signal })
      .then((r) => { if (!r.ok) throw new Error('No se pudieron cargar las frases'); return r.text() })
      .then((text) => {
        if (!alive) return
        const phrases = text.split(/\r?\n/).filter((p) => p && p.length <= 36 && !p.includes('§'))
        if (phrases.length) splash = phrases[Math.floor(Math.random() * phrases.length)]
      })
      .catch(() => {})
    return () => {
      alive = false
      controller.abort()
      stopAnimation()
    }
  })

  function quit() {
    getCurrentWindow().close()
  }
</script>

<div class="mc-screen">
  <header class="title-zone">
    <h1 class="sr-only">DisorderLauncher</h1>
    <canvas class="logo" bind:this={logo} style="top:{logoTop}px;width:{logoWidth}px;height:{logoHeight}px" aria-hidden="true"></canvas>
    <canvas class="splash" bind:this={splashEl} style="left:{splashBox.x}px;top:{splashBox.y}px;width:{splashBox.width}px;height:{splashBox.height}px" aria-hidden="true"></canvas>
  </header>

  {#each notes as note, i (note.text)}
    <McLabel center y={k - 12 - (notes.length - 1 - i) * 10} color={note.color} max={gui.W - 8} text={note.text} />
  {/each}

  <nav aria-label="Menú principal">
    <McButton x={menuX} w={menuW} label="Mis instancias" y={k} onintent={() => nav.prepare('instances')} onclick={() => nav.go('instances')} />
    <McButton x={menuX} w={menuW} label="Explorar" y={k + 24} onclick={() => nav.go('explore')} />
    <McButton x={menuX} w={menuW} label="Herramientas" y={k + 48} onclick={() => nav.go('tools')} />
    <McButton x={menuX} w={menuW} label="Perfiles" y={k + 72} onintent={() => nav.prepare('accounts')} onclick={() => nav.go('accounts')} />
    <button type="button" class="mc-language" style="{posStyle(-gui.cx + 6, 6)}" aria-label="Ayuda y soporte" aria-haspopup="dialog" onclick={onhelp}></button>
    <McButton label="Ajustes..." ariaLabel="Ajustes" x={menuX} y={k + 96} w={Math.floor(menuW / 2) - 2} onclick={() => nav.go('settings')} />
    <McButton label="Salir" x={menuX + Math.floor(menuW / 2) + 2} y={k + 96} w={Math.floor(menuW / 2) - 2} disabled={!inTauri} onclick={quit} />
  </nav>

  <McLabel center x={skinX + skinW / 2} y={k} max={skinW} text={account?.label ?? 'Jugador'} />
  <McViewport x={skinX} y={k + 10} w={skinW} h={72} frame={false}>
    <SkinPreview3D src={skinSrc} model={asset?.model ?? 'classic'} height={asset?.height ?? 64} unit={2} />
  </McViewport>
  {#if !skinId}<McLabel center x={skinX + skinW / 2} y={k + 84} max={skinW} color="#a0a0a0" text="Sin skin local" />{/if}
  <McButton x={skinX} y={k + 96} w={skinW} label="Skins" onclick={() => nav.go('skins')} />
  {#if tour && tourAllowed}
    <McDialog topAligned {highlight} title={`${step + 1}/${TOUR.length} · ${TOUR[step][0]}`} message={TOUR[step][1]}
      buttons={[{ label: 'Saltar', onclick: closeTour },
        { label: step ? 'Anterior' : 'Ayuda', onclick: () => { if (step) step--; else { closeTour(); onhelp() } } },
        { label: step === TOUR.length - 1 ? 'Terminar' : 'Siguiente', onclick: () => { if (step === TOUR.length - 1) closeTour(); else step++ } }]}
      cancelIndex={2} onclose={closeTour} />
  {/if}
  <footer>
    <span class="footer-text footer-left"><BitmapText text="DisorderLauncher {APP_VERSION}" /></span>
    <span class="footer-text footer-right"><BitmapText max={gui.W - 126} text="No afiliado a Mojang ni Microsoft" /></span>
  </footer>
</div>
