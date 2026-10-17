<script lang="ts">
  interface Option {
    value: string
    label: string
  }

  interface Props {
    label: string
    value: string
    options: Option[]
    disabled?: boolean
  }

  let { label, value = $bindable(), options, disabled = false }: Props = $props()
  const uid = $props.id()
</script>

<div class="field">
  <label for={uid}>{label}</label>
  <select id={uid} bind:value {disabled}>
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
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

  select {
    min-height: 44px;
    padding: 0 14px;
    border-radius: var(--r-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  select:disabled {
    opacity: 0.5;
  }
</style>
