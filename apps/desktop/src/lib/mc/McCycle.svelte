<script lang="ts" generics="T extends string | number">
  import McButton from './McButton.svelte'
  import type { Anchor } from './pos'

  interface Props {
    label: string
    options: { value: T; label: string }[]
    value: T
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
    disabled?: boolean
    onchange?: (value: T) => void
  }

  let { label, options, value = $bindable(), x = -100, y = 0, w = 200, anchor = 'top', disabled = false, onchange }: Props = $props()

  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)))
  const current = $derived(options[index]?.label ?? '')

  function step(delta: number) {
    if (disabled || options.length === 0) return
    const next = options[(index + delta + options.length) % options.length].value
    value = next
    onchange?.(next)
  }
</script>

<McButton
  label="{label}: {current}"
  ariaLabel="{label}: {current}. Pulsa para cambiar"
  {x}
  {y}
  {w}
  {anchor}
  {disabled}
  onclick={() => step(1)}
  extra={{
    oncontextmenu: (e: MouseEvent) => {
      e.preventDefault()
      step(-1)
    },
    onkeydown: (e: KeyboardEvent) => {
      if (e.key === 'ArrowRight') {
        e.preventDefault()
        step(1)
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault()
        step(-1)
      }
    },
  }}
/>
