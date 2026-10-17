<script lang="ts">
  import { blockTypes } from './types'

  let { type }: { type: string } = $props()

  type Shape = { d: string; fill: string }

  /** Objetos dibujados como SVG; los bloques usan el div `.block` del CSS. */
  const shapes: Record<string, Shape[]> = {
    diamond: [
      { d: "M9 3h14v4h4v12h-4v5h-4v5h-6v-5H9v-5H5V7h4Z", fill: "#346f70" },
      { d: "M10 6h12v4h3v7h-5v5h-4v4h-3v-7H8v-9h2Z", fill: "#7bc9c0" },
      { d: "M11 7h8v4h-5v7h-4v-7h1Z", fill: "#c3e7d7" },
    ],
    sword: [
      { d: "M22 2h8v8L16 24l-3-3-7 8H2v-4l8-8-3-3 3-3 5 3Z", fill: "#404944" },
      { d: "M24 4h4v5L15 22l-5-5 3-3 3 3Z", fill: "#b0d4cc" },
      { d: "m4 26 7-7 3 3-7 7H4Z", fill: "#8b7356" },
    ],
    apple: [
      { d: "M15 3h4v6h-4Z", fill: "#8b9a65" },
      { d: "M10 9h5v2h5V9h5v4h3v12h-4v4H9v-4H5V14h5Z", fill: "#a65e50" },
      { d: "M10 12h4v4h-4v8H8V15h2Z", fill: "#d39677" },
    ],
  }

  const selected = $derived(shapes[type] ?? shapes.apple)
</script>

{#if blockTypes.includes(type)}
  <div class="block {type}">
    <span></span>
    <i></i>
    <b></b>
  </div>
{:else}
  <svg
    class="item-logo {type}"
    viewBox="0 0 32 32"
    width="38"
    height="38"
    aria-hidden="true"
  >
    {#each selected as shape, i (i)}
      <path d={shape.d} fill={shape.fill} />
    {/each}
  </svg>
{/if}