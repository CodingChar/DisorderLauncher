<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    /** Desde el centro horizontal. */
    x: number
    y: number
    w: number
    h: number
    /** Marco de 1 px y fondo oscuro. */
    frame?: boolean
    children: Snippet
  }

  let { x, y, w, h, frame = true, children }: Props = $props()
</script>

<div class="mc-viewport" class:bare={!frame} style="left:calc(var(--cx) * 1px + {x}px);top:{y}px;width:{w}px;height:{h}px">
  {@render children()}
</div>
