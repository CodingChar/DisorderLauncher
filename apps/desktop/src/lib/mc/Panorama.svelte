<script lang="ts">
  import { onMount } from 'svelte'

  let panorama = $state<HTMLDivElement>()
  let cube = $state<HTMLDivElement>()

  const distance = () => Math.round(Math.max(innerWidth, innerHeight) / 2 / 0.81)

  function place(ticks: number) {
    if (!cube) return
    const pitch = Math.sin(ticks / 400) * 25 + 20
    cube.style.transform = `translateZ(${distance()}px) rotateX(${-pitch}deg) rotateY(${-ticks * 0.1}deg)`
  }

  onMount(() => {
    const motion = matchMedia('(prefers-reduced-motion: reduce)')
    let raf = 0
    const frame = (now: number) => {
      place(now / 50)
      raf = requestAnimationFrame(frame)
    }
    const resize = () => {
      if (panorama) panorama.style.perspective = `${distance()}px`
      place(0)
    }
    // Sin animacion si la persona la desactivo; en pausa con la ventana oculta o minimizada (0 % de CPU).
    const sync = () => {
      cancelAnimationFrame(raf)
      if (!motion.matches && !document.hidden) raf = requestAnimationFrame(frame)
      else place(0)
    }
    resize()
    sync()
    addEventListener('resize', resize, { passive: true })
    document.addEventListener('visibilitychange', sync)
    motion.addEventListener('change', sync)
    return () => {
      cancelAnimationFrame(raf)
      removeEventListener('resize', resize)
      document.removeEventListener('visibilitychange', sync)
      motion.removeEventListener('change', sync)
    }
  })
</script>

<div class="panorama-wrap" aria-hidden="true">
  <div class="panorama" bind:this={panorama}>
    <div class="pano-cube" bind:this={cube}>
      {#each [0, 1, 2, 3, 4, 5] as face (face)}<div class="pano-face pano-{face}"></div>{/each}
    </div>
  </div>
  <div class="panorama-shade"></div>
</div>
