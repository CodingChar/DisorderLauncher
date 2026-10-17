<script lang="ts">
  import { loaderLabel } from '../text'
  import type { Instance } from '../types'

  interface Props {
    instance: Instance
    selected: boolean
    onselect: () => void
  }

  let { instance, selected, onselect }: Props = $props()
</script>

<button class="card" class:selected aria-pressed={selected} onclick={onselect}>
  <strong>{instance.name}</strong>
  <small>{loaderLabel(instance.loader)} {instance.minecraftVersion}</small>
</button>

<style>
  .card {
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 64px;
    padding: 12px 16px;
    border-radius: var(--r-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-primary);
    transition: border-color 0.18s, background 0.18s;
  }

  .card:hover {
    background: var(--surface-alt);
  }

  .selected {
    border-color: var(--action);
    box-shadow: inset 3px 0 0 var(--action);
  }

  strong {
    font-family: var(--font-display);
    letter-spacing: -0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  small {
    color: var(--text-secondary);
  }
</style>
