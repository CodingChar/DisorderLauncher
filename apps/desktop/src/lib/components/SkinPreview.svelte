<script lang="ts">
  import { FRONT_SIZE, frontParts, type Part } from '../skinGeometry'
  import type { SkinModel } from '../types'

  interface Props {
    /** Data URL del PNG de la skin. */
    src: string
    model: SkinModel
    /** Pixeles de pantalla por pixel de skin. */
    scale?: number
    label: string
  }

  let { src, model, scale = 6, label }: Props = $props()

  let canvas = $state<HTMLCanvasElement>()

  $effect(() => {
    if (!canvas || !src) return
    const target = canvas
    const image = new Image()
    let cancelled = false
    image.onload = () => {
      if (cancelled) return
      const ctx = target.getContext('2d')
      if (!ctx) return
      ctx.imageSmoothingEnabled = false
      ctx.clearRect(0, 0, target.width, target.height)
      const legacy = image.height === 32
      const { base, overlay } = frontParts(model, legacy)
      for (const part of [...base, ...overlay]) draw(ctx, image, part)
    }
    image.src = src
    return () => {
      cancelled = true
    }
  })

  function draw(ctx: CanvasRenderingContext2D, image: HTMLImageElement, p: Part) {
    ctx.save()
    if (p.flip) {
      // Espejo horizontal dentro de la caja de destino.
      ctx.translate((p.dx + p.sw) * scale + p.dx * scale, 0)
      ctx.scale(-1, 1)
    }
    ctx.drawImage(image, p.sx, p.sy, p.sw, p.sh, p.dx * scale, p.dy * scale, p.sw * scale, p.sh * scale)
    ctx.restore()
  }
</script>

<div class="wrap" role="img" aria-label={label}>
  <canvas bind:this={canvas} width={FRONT_SIZE.w * scale} height={FRONT_SIZE.h * scale} aria-hidden="true"></canvas>
</div>

<style>
  .wrap {
    display: contents;
  }

  canvas {
    image-rendering: pixelated;
    width: calc(var(--w, 96px));
    max-width: 100%;
    height: auto;
    aspect-ratio: 1 / 2;
    background: repeating-conic-gradient(var(--surface-alt) 0% 25%, var(--surface) 0% 50%) 50% / 12px 12px;
    border-radius: var(--r-sm);
  }
</style>
