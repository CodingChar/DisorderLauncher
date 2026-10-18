<script lang="ts" generics="T extends string">
  import McButton from './McButton.svelte'

  interface Props {
    tabs: { id: T; label: string }[]
    value: T
    x?: number
    y?: number
    w?: number
    disabled?: boolean
    onchange?: (id: T) => void
  }

  let { tabs, value = $bindable(), x = -100, y = 0, w = 200, disabled = false, onchange }: Props = $props()

  const GAP = 4
  const widths = $derived.by(() => {
    const each = Math.floor((w - GAP * (tabs.length - 1)) / tabs.length)
    return tabs.map((_, i) => (i === tabs.length - 1 ? w - (each + GAP) * (tabs.length - 1) : each))
  })
  const offsets = $derived(widths.map((_, i) => widths.slice(0, i).reduce((sum, width) => sum + width + GAP, 0)))

  function select(id: T) {
    if (id === value) return
    value = id
    onchange?.(id)
  }

  function key(event: KeyboardEvent, i: number) {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return
    event.preventDefault()
    const next = tabs[(i + (event.key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length]
    select(next.id)
    queueMicrotask(() => document.querySelector<HTMLElement>(`[data-tab="${next.id}"]`)?.focus())
  }
</script>

<div role="tablist" style="display: contents">
  {#each tabs as tab, i (tab.id)}
    <McButton
      label={tab.label}
      x={x + offsets[i]}
      {y}
      w={widths[i]}
      {disabled}
      active={tab.id === value}
      onclick={() => select(tab.id)}
      extra={{ role: 'tab', 'aria-selected': tab.id === value, 'data-tab': tab.id, tabindex: tab.id === value ? 0 : -1, onkeydown: (e: KeyboardEvent) => key(e, i) }}
    />
  {/each}
</div>
