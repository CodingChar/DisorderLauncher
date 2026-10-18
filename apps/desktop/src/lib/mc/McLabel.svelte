<script lang="ts">
  import BitmapText from './BitmapText.svelte'
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
</script>

<div class="mc-label" style={posStyle(x, y, anchor)}><BitmapText {text} {color} {center} {max} /></div>
