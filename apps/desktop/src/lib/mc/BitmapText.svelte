<script lang="ts">
  import { fitText, renderText, stringWidth } from './font'
  import { fontStatus } from './fontStatus.svelte'

  interface Props {
    text?: string
    color?: string
    /** Ancho maximo en px de GUI (0 = sin limite): lo que no cabe se acorta con «...». */
    max?: number
    /** Centra sobre su origen (division entera, como Minecraft). */
    center?: boolean
    /** Texto accesible oculto. Los controles que ya llevan `aria-label` lo desactivan. */
    sr?: boolean
  }

  let { text = '', color = '#ffffff', max = 0, center = false, sr = true }: Props = $props()

  let canvas = $state<HTMLCanvasElement>()

  const shown = $derived(fitText(text ?? '', max ? max - 1 : 0)) // el lienzo anade 1 px de sombra
  const width = $derived(stringWidth(shown))

  $effect(() => {
    if (!canvas || !fontStatus.ready) return
    const image = renderText(shown, color)
    canvas.width = image.width
    canvas.height = image.height
    canvas.getContext('2d')!.drawImage(image, 0, 0)
  })
</script>

<span class="bitmap-label" style:margin-left={center ? `${-(width >> 1)}px` : undefined}>{#if sr}<span class="sr-only">{text}</span>{/if}<canvas bind:this={canvas} width={Math.max(1, width + 1)} height="9" aria-hidden="true"></canvas></span>
