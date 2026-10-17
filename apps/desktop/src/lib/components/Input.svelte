<script lang="ts">
  interface Props {
    label: string
    value: string
    hint?: string
    error?: string | null
    placeholder?: string
    maxlength?: number
    autofocus?: boolean
    onenter?: () => void
  }

  let { label, value = $bindable(), hint, error = null, placeholder, maxlength, autofocus = false, onenter }: Props = $props()

  const uid = $props.id()
  const describedBy = $derived(error ? `${uid}-error` : hint ? `${uid}-hint` : undefined)

  function focusOnMount(node: HTMLInputElement) {
    if (autofocus) node.focus()
  }
</script>

<div class="field">
  <label for={uid}>{label}</label>
  <input
    id={uid}
    bind:value
    {placeholder}
    {maxlength}
    autocomplete="off"
    spellcheck="false"
    aria-invalid={error ? 'true' : undefined}
    aria-describedby={describedBy}
    use:focusOnMount
    onkeydown={(e) => e.key === 'Enter' && onenter?.()}
  />
  {#if error}
    <p class="error" id="{uid}-error" role="alert">{error}</p>
  {:else if hint}
    <p class="hint" id="{uid}-hint">{hint}</p>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  input {
    min-height: 44px;
    padding: 0 14px;
    border-radius: var(--r-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  input[aria-invalid='true'] {
    border-color: var(--text-primary);
    border-style: dashed;
  }

  p {
    margin: 0;
    font-size: 12px;
  }

  .hint {
    color: var(--text-secondary);
  }

  .error {
    color: var(--text-primary);
    font-weight: 600;
  }
</style>
