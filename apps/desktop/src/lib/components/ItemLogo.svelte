<script lang="ts">
  import { findItemIcon } from '../itemIcons'
  import type { LoaderKind } from '../types'

  /** `icon`: id elegido por la persona; sin el, se usa el logo del tipo de instancia. */
  let { loader, icon = null, size = 38 }: { loader: LoaderKind; icon?: string | null; size?: number } = $props()

  const chosen = $derived(findItemIcon(icon))

  type Shape = { d: string; fill: string }

  /** Cada cargador se identifica con un bloque (CSS `.block`) u objeto (SVG). */
  const blockOf: Partial<Record<LoaderKind, string>> = { vanilla: 'grass', fabric: 'sand', forge: 'stone' }

  const shapes: Record<string, Shape[]> = {
    diamond: [
      { d: 'M9 3h14v4h4v12h-4v5h-4v5h-6v-5H9v-5H5V7h4Z', fill: '#346f70' },
      { d: 'M10 6h12v4h3v7h-5v5h-4v4h-3v-7H8v-9h2Z', fill: '#7bc9c0' },
      { d: 'M11 7h8v4h-5v7h-4v-7h1Z', fill: '#c3e7d7' },
    ],
    apple: [
      { d: 'M15 3h4v6h-4Z', fill: '#8b9a65' },
      { d: 'M10 9h5v2h5V9h5v4h3v12h-4v4H9v-4H5V14h5Z', fill: '#a65e50' },
      { d: 'M10 12h4v4h-4v8H8V15h2Z', fill: '#d39677' },
    ],
  }

  const block = $derived(blockOf[loader])
  const item = $derived(shapes[loader === 'neoforge' ? 'diamond' : 'apple'])
</script>

{#if chosen}
  <svg class="item-logo" viewBox="0 0 16 16" width={size} height={size} shape-rendering="crispEdges" aria-hidden="true">
    {#each chosen.pixels as color, i (i)}
      {#if color}<rect x={i % 16} y={Math.floor(i / 16)} width="1" height="1" fill={color} />{/if}
    {/each}
  </svg>
{:else if block}
  <div class="block {block}" aria-hidden="true">
    <span></span>
    <i></i>
    <b></b>
  </div>
{:else}
  <svg class="item-logo" viewBox="0 0 32 32" width={size} height={size} aria-hidden="true">
    {#each item as shape, i (i)}
      <path d={shape.d} fill={shape.fill} />
    {/each}
  </svg>
{/if}
