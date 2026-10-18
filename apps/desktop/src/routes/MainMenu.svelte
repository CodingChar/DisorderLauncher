<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import BitmapText from '../lib/mc/BitmapText.svelte'
  import McButton from '../lib/mc/McButton.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import { stringWidth } from '../lib/mc/font'
  import { useGui } from '../lib/mc/gui'
  import { drawLogo } from '../lib/mc/logo'
  import { posStyle } from '../lib/mc/pos'
  import { nav } from '../lib/stores/nav.svelte'
  import { APP_VERSION } from '../lib/version'

  interface Props {
    online: boolean
    /** Error al cambiar de tema (se muestra bajo el logo). */
    themeError: string | null
    onhelp: () => void
  }

  let { online, themeError, onhelp }: Props = $props()

  const gui = useGui()
  const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

  let logo = $state<HTMLCanvasElement>()
  let splashEl = $state<HTMLDivElement>()
  let logoWidth = $state(288)
  let logoHeight = $state(28)
  let splash = $state('¡Bienvenido!')

  // Y del primer boton: height / 4 + 48 (GuiMainMenu).
  const k = $derived((gui.H >> 2) + 48)
  const logoTop = $derived(30 + Math.round((44 - logoHeight) / 2))
  const splashWidth = $derived(stringWidth(splash))
  const notes = $derived(
    [themeError ? { text: themeError, color: '#ff5555' } : null, !online ? { text: 'Sin conexión: puedes jugar lo que ya descargaste', color: '#a0a0a0' } : null].filter(
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

  function placeSplash(now: number, still: boolean) {
    if (!splashEl) return
    const pulse = still ? 0 : Math.abs(Math.sin(((now % 1000) / 1000) * Math.PI * 2) * 0.1)
    const factor = ((1.8 - pulse) * 100) / (splashWidth + 32)
    splashEl.style.transform = `translate(${gui.cx + logoWidth / 2 - 47}px, ${logoTop + logoHeight - 4}px) rotate(-20deg) scale(${factor}) translate(${-(splashWidth >> 1)}px, -8px)`
  }

  onMount(() => {
    const motion = matchMedia('(prefers-reduced-motion: reduce)')
    let raf = 0
    let alive = true
    const loop = (now: number) => {
      placeSplash(now, false)
      raf = requestAnimationFrame(loop)
    }
    const sync = () => {
      cancelAnimationFrame(raf)
      if (!motion.matches && !document.hidden) raf = requestAnimationFrame(loop)
      else placeSplash(0, true)
    }
    sync()
    document.addEventListener('visibilitychange', sync)
    motion.addEventListener('change', sync)
    fetch('/assets/splashes.txt')
      .then((r) => r.text())
      .then((text) => {
        if (!alive) return
        const phrases = text.split(/\r?\n/).filter((p) => p && p.length <= 36 && !p.includes('§'))
        if (phrases.length) splash = phrases[Math.floor(Math.random() * phrases.length)]
      })
      .catch(() => {})
    return () => {
      alive = false
      cancelAnimationFrame(raf)
      document.removeEventListener('visibilitychange', sync)
      motion.removeEventListener('change', sync)
    }
  })

  // El escenario no se reencaja solo: la posicion del splash depende del logo y de la ventana.
  $effect(() => {
    void [gui.W, gui.cx, logoWidth, logoHeight, splashWidth]
    if (matchMedia('(prefers-reduced-motion: reduce)').matches || document.hidden) placeSplash(0, true)
  })

  function quit() {
    getCurrentWindow().close()
  }
</script>

<div class="mc-screen">
  <header class="title-zone">
    <h1 class="sr-only">DisorderLauncher</h1>
    <canvas class="logo" bind:this={logo} style="top:{logoTop}px;width:{logoWidth}px;height:{logoHeight}px" aria-hidden="true"></canvas>
    <div class="splash" bind:this={splashEl} aria-hidden="true"><BitmapText text={splash} color="#ffff00" sr={false} /></div>
  </header>

  {#each notes as note, i (note.text)}
    <McLabel center y={k - 12 - (notes.length - 1 - i) * 10} color={note.color} max={gui.W - 8} text={note.text} />
  {/each}

  <nav aria-label="Menú principal">
    <McButton label="Mis instancias" y={k} onclick={() => nav.go('instances')} />
    <McButton label="Explorar" y={k + 24} onclick={() => nav.go('explore')} />
    <McButton label="Herramientas" y={k + 48} onclick={() => nav.go('tools')} />
    <McButton label="Perfiles y skins" y={k + 72} onclick={() => nav.go('accounts')} />
    <button type="button" class="mc-language" style="{posStyle(-124, k + 96)}" aria-label="Ayuda y soporte" aria-haspopup="dialog" onclick={onhelp}></button>
    <McButton label="Ajustes..." ariaLabel="Ajustes" x={-100} y={k + 96} w={98} onclick={() => nav.go('settings')} />
    <McButton label="Salir" x={2} y={k + 96} w={98} disabled={!inTauri} onclick={quit} />
  </nav>

  <footer>
    <span class="footer-text footer-left"><BitmapText text="DisorderLauncher {APP_VERSION}" /></span>
    <span class="footer-text footer-right"><BitmapText max={gui.W - 126} text="No afiliado a Mojang ni Microsoft" /></span>
  </footer>
</div>
