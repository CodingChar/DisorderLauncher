<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import Panorama from './Panorama.svelte'
  import { provideGui, type Gui } from './gui'

  interface Props {
    /** Fondo: panorama 3D (menu principal) o tierra de las pantallas de opciones. */
    panorama?: boolean
    children: Snippet
  }

  let { panorama = false, children }: Props = $props()

  // En DEV `window.__forceGui = { w: 320, h: 240 }` fuerza el tamano virtual (pruebas de overflow; ver src/mc/globals.d.ts).

  const gui: Gui = $state({ W: 410, H: 280, cx: 205, s: 2, factor: 2 })
  provideGui(gui)

  function layout() {
    const dpr = window.devicePixelRatio || 1
    const deviceW = Math.round(innerWidth * dpr)
    const deviceH = Math.round(innerHeight * dpr)
    // Factor entero de GUI en px de dispositivo: nitidez perfecta con cualquier DPI.
    const factor = Math.max(1, Math.floor(Math.min(deviceW / 320, deviceH / 240)))
    const forced = import.meta.env.DEV ? window.__forceGui : undefined
    gui.factor = factor
    gui.s = factor / dpr
    gui.W = forced?.w ?? Math.ceil(deviceW / factor)
    gui.H = forced?.h ?? Math.ceil(deviceH / factor)
    gui.cx = gui.W >> 1
  }

  layout()

  onMount(() => {
    layout()
    addEventListener('resize', layout, { passive: true })
    return () => removeEventListener('resize', layout)
  })
</script>

{#if panorama}
  <Panorama />
{:else}
  <div class="dirt-wrap" aria-hidden="true"></div>
{/if}

<main class="mc-stage" style="--s:{gui.s};--W:{gui.W};--H:{gui.H};--cx:{gui.cx}" aria-label="DisorderLauncher">
  {@render children()}
</main>
