<script lang="ts">
  /**
   * Iconos dibujados con `currentColor`, sin dependencias externas.
   * Cada icono es una lista ordenada de partes SVG.
   */
  type Part = {
    s: "p" | "c" | "r"
    d?: string
    fill?: string
    cx?: number
    cy?: number
    r?: number
    x?: number
    y?: number
    w?: number
    h?: number
    rx?: number
  }

  const p = (d: string): Part => ({ s: "p", d })
  const solid = (d: string): Part => ({ s: "p", d, fill: "currentColor" })
  const c = (cx: number, cy: number, r: number): Part => ({ s: "c", cx, cy, r })
  const rect = (x: number, y: number, w: number, h: number, rx?: number): Part => ({
    s: "r",
    x,
    y,
    w,
    h,
    rx,
  })

  const icons: Record<string, Part[]> = {
    home: [p("m3 10 9-7 9 7v10H3Z"), p("M9 20v-7h6v7")],
    box: [p("m12 3 9 5v9l-9 5-9-5V8Z"), p("m3 8 9 5 9-5M12 13v9M7 5l10 6")],
    compass: [c(12, 12, 9), p("m16 8-3 5-5 3 3-5Z")],
    user: [c(12, 8, 4), p("M4 21v-2a8 8 0 0 1 16 0v2")],
    settings: [
      p("m9 3-1 3-3 1-2 4 2 3v4l4 2 3-1 3 1 4-2v-4l2-3-2-4-3-1-1-3Z"),
      c(12, 12, 3),
    ],
    arrow: [p("M5 12h14m-5-5 5 5-5 5")],
    chevron: [p("m8 10 4 4 4-4")],
    plus: [p("M12 5v14M5 12h14")],
    play: [solid("m8 5 11 7-11 7Z")],
    sun: [
      c(12, 12, 4),
      p("M12 2v2m0 16v2M2 12h2m16 0h2M5 5l2 2m10 10 2 2M5 19l2-2M17 7l2-2"),
    ],
    moon: [p("M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11Z")],
    check: [p("m5 12 4 4L19 6")],
    memory: [
      rect(5, 5, 14, 14, 2),
      p("M9 9h6v6H9ZM8 2v3m8-3v3M8 19v3m8-3v3M2 8h3m14 0h3M2 16h3m14 0h3"),
    ],
    grid: [
      rect(3, 3, 7, 7, 1),
      rect(14, 3, 7, 7, 1),
      rect(3, 14, 7, 7, 1),
      rect(14, 14, 7, 7, 1),
    ],
    help: [c(12, 12, 9), p("M9 9a3 3 0 1 1 4 3v2m0 3h.01")],
    close: [p("m6 6 12 12M6 18 18 6")],
    shirt: [p("m9 3 3 3 3-3 5 2-2 5-2-1v11H8V9l-2 1-2-5Z")],
    wrench: [
      p(
        "M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.8-3.8a6 6 0 0 1-7.9 7.9l-6.9 6.9a2.1 2.1 0 0 1-3-3l6.9-6.9a6 6 0 0 1 7.9-7.9Z",
      ),
    ],
    trash: [p("M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6")],
    archive: [p("M3 4h18v5H3zM5 9v11h14V9M10 13h4")],
    clipboard: [p("M9 4H7v17h10V4h-2M9 2h6v3H9zM11 11h2M11 15h2")],
    zap: [p("M13 2 4 14h7l-1 8 9-12h-7Z")],
  }

  let { name, size = 20 }: { name: string; size?: number } = $props()

  const parts = $derived(icons[name] ?? icons.box)
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each parts as part, i (i)}
    {#if part.s === 'c'}
      <circle cx={part.cx} cy={part.cy} r={part.r} />
    {:else if part.s === 'r'}
      <rect
        x={part.x}
        y={part.y}
        width={part.w}
        height={part.h}
        rx={part.rx ?? 0}
      />
    {:else}
      <path
        d={part.d}
        fill={part.fill ?? 'none'}
        stroke={part.fill ? 'none' : 'currentColor'}
      />
    {/if}
  {/each}
</svg>