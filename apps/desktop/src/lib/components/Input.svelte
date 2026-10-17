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
    <p class="field-error" id="{uid}-error" role="alert">{error}</p>
  {:else if hint}
    <p class="field-help" id="{uid}-hint">{hint}</p>
  {/if}
</div>
