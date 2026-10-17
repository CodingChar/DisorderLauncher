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
