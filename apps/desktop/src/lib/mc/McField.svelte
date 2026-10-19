<script lang="ts">
  import { onMount } from 'svelte'
  import BitmapText from './BitmapText.svelte'
  import McText from './McText.svelte'
  import { stringWidth, tailText } from './font'
  import { posStyle, type Anchor } from './pos'

  interface Props {
    value?: string
    placeholder?: string
    /** Nombre accesible (por defecto, el texto de ejemplo). */
    label?: string
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
    maxlength?: number
    type?: 'text' | 'password'
    disabled?: boolean
    autofocus?: boolean
    /** Mensaje de error: borde rojo y texto debajo. */
    error?: string | null
    /** Ayuda en gris debajo del campo. */
    hint?: string
    onenter?: () => void
    oninput?: (value: string) => void
  }

  let {
    value = $bindable(''),
    placeholder = '',
    label = '',
    x = -100,
    y = 0,
    w = 200,
    anchor = 'top',
    maxlength = 60,
    type = 'text',
    disabled = false,
    autofocus = false,
    error = null,
    hint = '',
    onenter,
    oninput,
  }: Props = $props()

  const uid = $props.id()
  let input = $state<HTMLInputElement>()
  let focused = $state(false)
  let caret = $state(true)
  let allSelected = $state(false)

  $effect(() => {
    if (!focused) return
    const timer = setInterval(() => (caret = !caret), 500)
    return () => clearInterval(timer)
  })

  onMount(() => {
    if (autofocus) input?.focus()
  })

  function syncSelection() {
    allSelected = !!input && input.value.length > 0 && input.selectionStart === 0 && input.selectionEnd === input.value.length
  }

  function onFocus() {
    focused = true
    caret = true
  }

  const text = $derived(type === 'password' ? '*'.repeat([...value].length) : String(value ?? ''))
  const shown = $derived(tailText(text + (focused && caret && !allSelected ? '_' : ''), w - 10))
  const boxStyle = $derived(`${posStyle(x, y, anchor)};width:${w}px`)
  const hasBelow = $derived(!!error || !!hint)
</script>

<div class="mc-field" class:focused class:invalid={!!error} class:is-disabled={disabled} style={boxStyle}>
  <input
    bind:this={input}
    bind:value
    {type}
    {maxlength}
    {disabled}
    aria-label={label || placeholder}
    aria-invalid={error ? true : undefined}
    aria-describedby={hasBelow ? `${uid}-help` : undefined}
    autocomplete="off"
    spellcheck="false"
    oninput={() => {
      allSelected = false
      oninput?.(value)
    }}
    onfocus={onFocus}
    onblur={() => {
      focused = false
      allSelected = false
    }}
    onselect={syncSelection}
    onkeyup={syncSelection}
    onmouseup={syncSelection}
    onkeydown={(e) => {
      if (e.key === 'Enter' && onenter) {
        e.preventDefault()
        onenter()
      }
    }}
  />
  {#if allSelected}<span class="selection" style:width="{Math.min(w - 8, stringWidth(shown) + 1)}px"></span>{/if}
  <span class="field-text">
    {#if !text && !focused && placeholder}<BitmapText text={placeholder} color="#707070" max={w - 10} sr={false} />
    {:else}<BitmapText text={shown} color="#e0e0e0" sr={false} />{/if}
  </span>
</div>

{#if hasBelow}
  <div id="{uid}-help">
    {#if error}
      <McText text={error} x={x} y={anchor === 'bottom' ? y + 22 : y + 22} {anchor} {w} color="#ff5555" maxLines={2} role="alert" />
    {:else}
      <McText text={hint} x={x} y={y + 22} {anchor} {w} color="#808080" maxLines={2} />
    {/if}
  </div>
{/if}
