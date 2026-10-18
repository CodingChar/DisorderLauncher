<script lang="ts">
  import McButton from './McButton.svelte'
  import type { Anchor } from './pos'

  interface Props {
    label: string
    checked: boolean
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
    disabled?: boolean
    onchange?: (checked: boolean) => void
  }

  let { label, checked = $bindable(), x = -100, y = 0, w = 200, anchor = 'top', disabled = false, onchange }: Props = $props()
</script>

<McButton
  label="{label}: {checked ? 'Sí' : 'No'}"
  ariaLabel={label}
  {x}
  {y}
  {w}
  {anchor}
  {disabled}
  onclick={() => {
    checked = !checked
    onchange?.(checked)
  }}
  extra={{ role: 'switch', 'aria-checked': checked }}
/>
