<script lang="ts">
  import { onMount } from 'svelte'
  import { animateVisible } from './animation'

  let panorama = $state<HTMLDivElement>()
  let cube = $state<HTMLDivElement>()

  let distance = 800

  function place(ticks: number) {
    if (!cube) return
    const pitch = Math.sin(ticks / 400) * 25 + 20
    cube.style.transform = `translateZ(${distance}px) rotateX(${-pitch}deg) rotateY(${-ticks * 0.1}deg)`
  }

  onMount(() => {
    const resize = () => {
      distance = Math.round(Math.max(innerWidth, innerHeight) / 2 / 0.81)
      if (panorama) panorama.style.perspective = `${distance}px`
      place(0)
    }
    resize()
    // El fondo gira lentamente: 30 Hz bastan, sin seguir los 120/144/240 Hz del monitor.
    const stopAnimation = animateVisible((now, still) => place(still ? 0 : now / 50), 30)
    addEventListener('resize', resize, { passive: true })
    return () => {
      stopAnimation()
      removeEventListener('resize', resize)
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
