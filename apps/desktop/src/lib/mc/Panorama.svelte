<script lang="ts">
  import { onMount } from 'svelte'
  import { panoramaMoves } from './motion'

  // El giro es una animacion CSS (compositor/GPU): sin JavaScript por fotograma, a la tasa del monitor.
  // Formula del menu original (RotatingCubeMapRenderer, 20 ticks por segundo):
  //   yaw   = -ticks * 0.1            -> 2 grados por segundo, vuelta completa en 180 s
  //   pitch = sin(ticks / 400) * 25 + 20 -> oscila entre -5 y 45 grados, periodo de 125,7 s
  let wrap = $state<HTMLDivElement>()
  const moves = panoramaMoves()
  let paused = $state(false)

  onMount(() => {
    const resize = () => {
      // Distancia de la camara para un campo de vision vertical equivalente al original.
      const distance = Math.round(Math.max(innerWidth, innerHeight) / 2 / 0.81)
      wrap?.style.setProperty('--pano-d', `${distance}px`)
    }
    // Pestana oculta o ventana minimizada: se detiene en lugar de gastar GPU sin que nadie lo vea.
    const sync = () => (paused = document.hidden)
    resize()
    sync()
    addEventListener('resize', resize, { passive: true })
    document.addEventListener('visibilitychange', sync)
    return () => {
      removeEventListener('resize', resize)
      document.removeEventListener('visibilitychange', sync)
    }
  })
</script>

<div class="panorama-wrap" class:still={!moves} class:paused aria-hidden="true" bind:this={wrap}>
  <div class="panorama">
    <div class="pano-pitch">
      <div class="pano-cube">
        {#each [0, 1, 2, 3, 4, 5] as face (face)}<div class="pano-face pano-{face}"></div>{/each}
      </div>
    </div>
  </div>
  <div class="panorama-shade"></div>
</div>
