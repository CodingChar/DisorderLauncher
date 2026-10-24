<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { fitText, stringWidth } from './font'
  import { palette } from './palette.svelte'
  import { posStyle, type Anchor } from './pos'
  import { playSfx } from '../audio/sfx'

  interface Props {
    value: number
    min?: number
    max?: number
    step?: number
    /** Texto del deslizador; recibe el valor actual. */
    format?: (value: number) => string
    /** Nombre accesible (por defecto, el texto formateado). */
    label?: string
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
    disabled?: boolean
    onchange?: (value: number) => void
  }

  let { value = $bindable(), min = 1, max = 16, step = 1, format = (v: number) => String(v), label, x = -100, y = 0, w = 200, anchor = 'top', disabled = false, onchange }: Props = $props()

  let dragging = false

  const text = $derived(format(value))
  const shown = $derived(fitText(text, w - 8))
  const left = $derived((w >> 1) - (stringWidth(shown) >> 1))
  const handle = $derived(Math.round((Math.max(min, Math.min(max, value)) - min) / (max - min || 1) * (w - 8)))

  const clamp = (n: number) => Math.max(min, Math.min(max, n))
  const snap = (n: number) => clamp(Math.round((n - min) / step) * step + min)

  function set(next: number) {
    if (next === value) return
    value = next
    playSfx('scroll')
    onchange?.(next)
  }

  function move(event: PointerEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const px = (event.clientX - rect.left) / (rect.width / w)
    const ratio = Math.max(0, Math.min(1, (px - 4) / (w - 8)))
    set(snap(min + ratio * (max - min)))
  }

  function key(event: KeyboardEvent) {
    if (disabled) return
    if (event.key === 'ArrowLeft' || event.key === 'ArrowDown') set(clamp(value - step))
    else if (event.key === 'ArrowRight' || event.key === 'ArrowUp') set(clamp(value + step))
    else if (event.key === 'Home') set(min)
    else if (event.key === 'End') set(max)
    else return
    event.preventDefault()
  }
</script>

<div
  class="mc-button mc-slider"
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label ?? text}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={value}
  aria-valuetext={text}
  aria-disabled={disabled || undefined}
  style="{posStyle(x, y, anchor)};width:{w}px;--w:{w}"
  onpointerdown={(e) => {
    if (disabled) return
    dragging = true
    e.currentTarget.setPointerCapture(e.pointerId)
    move(e)
  }}
  onpointermove={(e) => dragging && move(e)}
  onpointerup={() => (dragging = false)}
  onpointercancel={() => (dragging = false)}
  onkeydown={key}
>
  <i class="handle" style:left="{handle}px"></i>
  <span class="caption" style:left="{left}px">
    <span class="c-normal"><BitmapText text={shown} color={disabled ? '#a0a0a0' : '#ffffff'} sr={false} /></span>
    <span class="c-hover"><BitmapText text={shown} color={palette.hover} sr={false} /></span>
  </span>
</div>
