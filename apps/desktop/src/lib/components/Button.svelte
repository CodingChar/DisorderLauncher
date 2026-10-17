<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost'
    type?: 'button' | 'submit'
    disabled?: boolean
    large?: boolean
    onclick?: (event: MouseEvent) => void
    children: Snippet
  }

  let { variant = 'secondary', type = 'button', disabled = false, large = false, onclick, children }: Props = $props()
</script>

<button class="btn {variant}" class:large {type} {disabled} {onclick}>
  {@render children()}
</button>

<style>
  .btn {
    min-height: 44px;
    padding: 0 20px;
    border-radius: var(--r-md);
    border: 1px solid var(--border-strong);
    background: var(--surface-alt);
    color: var(--text-primary);
    font-weight: 600;
    transition: background 0.18s, border-color 0.18s, opacity 0.18s;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-raised);
  }

  .primary {
    background: var(--action);
    color: var(--action-text);
    border-color: var(--action);
  }

  .primary:hover:not(:disabled) {
    background: var(--action);
    opacity: 0.88;
  }

  .ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
  }

  .ghost:hover:not(:disabled) {
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  .large {
    min-height: 56px;
    padding: 0 40px;
    font-size: 18px;
    font-family: var(--font-display);
    letter-spacing: 0.04em;
    border-radius: 0 var(--r-md) var(--r-md) 0;
  }

  .btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
