<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { fitText, stringWidth } from './font'
  import { palette } from './palette.svelte'
  import { posStyle, type Anchor } from './pos'

  interface Props {
    label: string
    /** Texto completo para lectores de pantalla (por defecto, `label`). */
    ariaLabel?: string
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
    disabled?: boolean
    /** Dibujado como pulsado (pestana o herramienta activa), sin estar deshabilitado. */
    active?: boolean
    type?: 'button' | 'submit'
    onclick?: (event: MouseEvent) => void
    /** Atributos extra (role, aria-*, eventos de teclado...). */
    extra?: Record<string, unknown>
  }

  let { label, ariaLabel, x = -100, y = 0, w = 200, anchor = 'top', disabled = false, active = false, type = 'button', onclick, extra = {} }: Props = $props()

  const shown = $derived(fitText(label, w - 8))
  // GuiButton centra con division entera: x + w / 2 - ancho / 2.
  const left = $derived((w >> 1) - (stringWidth(shown) >> 1))
</script>

<button
  {type}
  class="mc-button"
  class:is-active={active}
  style="{posStyle(x, y, anchor)};width:{w}px;--w:{w}"
  {disabled}
  {onclick}
  aria-label={ariaLabel ?? label}
  {...extra}
>
  <span class="caption" style:left="{left}px">
    <span class="c-normal"><BitmapText text={shown} sr={false} /></span>
    <span class="c-hover"><BitmapText text={shown} color={palette.hover} sr={false} /></span>
    {#if disabled}<span class="c-off"><BitmapText text={shown} color="#a0a0a0" sr={false} /></span>{/if}
    {#if active}<span class="c-active"><BitmapText text={shown} color="#ffffff" sr={false} /></span>{/if}
  </span>
</button>
