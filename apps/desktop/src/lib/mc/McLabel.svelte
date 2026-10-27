<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import McMore from './McMore.svelte'
  import { detail } from './detail.svelte'
  import { MORE_WIDTH, fitText, isTruncated, stringWidth } from './font'
  import { posStyle, type Anchor } from './pos'

  interface Props {
    text?: string
    color?: string
    /** Desde el centro horizontal. */
    x?: number
    y?: number
    anchor?: Anchor
    /** `center`: el texto queda centrado sobre `x`. */
    center?: boolean
    max?: number
  }

  let { text = '', color = '#ffffff', x = 0, y = 0, anchor = 'top', center = false, max = 0 }: Props = $props()

  // Si no cabe, se acorta dejando sitio a «[+]», que abre el texto entero en una ventana.
  const truncated = $derived(isTruncated(text, max))
  const fitted = $derived(truncated ? fitText(text, max - MORE_WIDTH) : text)
  const width = $derived(stringWidth(fitted))
  const moreX = $derived(center ? x + (width >> 1) + 2 : x + width + 2)
</script>

<div class="mc-label" style={posStyle(x, y, anchor)}><BitmapText text={fitted} {color} {center} max={truncated ? 0 : max} /></div>
{#if truncated}
  <McMore style={posStyle(moreX, y - 1, anchor)} label="Ver el texto completo" onclick={() => detail.show('Detalles', text)} />
{/if}
