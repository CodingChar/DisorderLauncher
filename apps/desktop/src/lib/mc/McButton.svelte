<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { fitText, stringWidth } from './font'
  import { palette } from './palette.svelte'
  import { posStyle, type Anchor } from './pos'
  import { playSfx, resolveSound, type SoundChoice } from '../audio/sfx'

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
    /** Sonido al pulsar: `auto` lo deduce de la etiqueta («Atras», «Volver»... suenan a volver). */
    sound?: SoundChoice
    /** Prepara los datos al acercar el puntero o enfocar, sin retrasar el clic. */
    onintent?: () => void
    /** Atributos extra (role, aria-*, eventos de teclado...). */
    extra?: Record<string, unknown>
  }

  let { label, ariaLabel, x = -100, y = 0, w = 200, anchor = 'top', disabled = false, active = false, type = 'button', sound = 'auto', onclick, onintent, extra = {} }: Props = $props()
  let hovered = $state(false)
  let keyboardFocus = $state(false)
  const color = $derived(disabled ? '#a0a0a0' : active ? '#ffffff' : hovered || keyboardFocus ? palette.hover : '#ffffff')

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
  onclick={(event) => {
    playSfx(resolveSound(sound, label))
    onclick?.(event)
  }}
  aria-label={ariaLabel ?? label}
  {...extra}
  onpointerenter={(event) => { hovered = true; if (!disabled) { playSfx('focus'); onintent?.() } if (typeof extra.onpointerenter === 'function') extra.onpointerenter(event) }}
  onpointerleave={(event) => { hovered = false; if (typeof extra.onpointerleave === 'function') extra.onpointerleave(event) }}
  onfocus={(event) => { keyboardFocus = event.currentTarget.matches(':focus-visible'); if (!disabled) { if (keyboardFocus) playSfx('focus'); onintent?.() } if (typeof extra.onfocus === 'function') extra.onfocus(event) }}
  onblur={(event) => { keyboardFocus = false; if (typeof extra.onblur === 'function') extra.onblur(event) }}
>
  <span class="caption" style:left="{left}px">
    <BitmapText text={shown} {color} sr={false} />
  </span>
</button>
