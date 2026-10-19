<script lang="ts">
  import { playSfx } from '../audio/sfx'
  import type { GridIcon } from './listTypes'

  interface Props {
    icons: GridIcon[]
    /** Id elegido. */
    value: string
    x: number
    y: number
    cols?: number
    ariaLabel: string
    disabled?: boolean
    onchange?: (id: string) => void
  }

  let { icons, value = $bindable(), x, y, cols = 5, ariaLabel, disabled = false, onchange }: Props = $props()

  const STRIDE = 38
  const rows = $derived(Math.ceil(icons.length / cols))
  const width = $derived(Math.min(cols, icons.length) * STRIDE - 2)
  const selectedIndex = $derived(Math.max(0, icons.findIndex((i) => i.id === value)))

  function pick(index: number) {
    const icon = icons[index]
    if (!icon || disabled) return
    value = icon.id
    onchange?.(icon.id)
  }

  function key(event: KeyboardEvent) {
    const delta: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: cols, ArrowUp: -cols }
    const step = delta[event.key]
    if (!step) return
    event.preventDefault()
    const next = Math.max(0, Math.min(icons.length - 1, selectedIndex + step))
    pick(next)
    queueMicrotask(() => document.querySelectorAll<HTMLElement>('.mc-icon-cell')[next]?.focus())
  }
</script>

<div class="mc-icon-grid" role="radiogroup" aria-label={ariaLabel} style="left:calc(var(--cx) * 1px - {width >> 1}px + {x}px);top:{y}px;width:{width}px;height:{rows * STRIDE - 2}px">
  {#each icons as icon, i (icon.id)}
    <button
      type="button"
      class="mc-icon-cell"
      class:selected={icon.id === value}
      role="radio"
      aria-checked={icon.id === value}
      aria-label={icon.label}
      tabindex={i === selectedIndex ? 0 : -1}
      {disabled}
      style="left:{(i % cols) * STRIDE}px;top:{Math.floor(i / cols) * STRIDE}px"
      onclick={() => {
        playSfx('click')
        pick(i)
      }}
      onkeydown={key}
    >
      <img src={icon.src} alt="" draggable="false" />
    </button>
  {/each}
</div>
