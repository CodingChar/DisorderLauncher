<script lang="ts">
  import { ITEM_ICONS } from '../itemIcons'
  import ItemLogo from './ItemLogo.svelte'
  import type { LoaderKind } from '../types'

  interface Props {
    /** Id del icono elegido; cadena vacia = el del tipo de instancia. */
    value: string
    loader: LoaderKind
    disabled?: boolean
  }

  let { value = $bindable(), loader, disabled = false }: Props = $props()
</script>

<fieldset class="icon-picker" {disabled}>
  <legend>Icono de la instancia</legend>
  <div class="icon-grid" role="radiogroup" aria-label="Icono de la instancia">
    <button type="button" class="icon-option" class:selected={value === ''} role="radio" aria-checked={value === ''} aria-label="Icono del tipo de instancia" title="Por defecto" onclick={() => (value = '')}>
      <ItemLogo {loader} size={28} />
    </button>
    {#each ITEM_ICONS as icon (icon.id)}
      <button type="button" class="icon-option" class:selected={value === icon.id} role="radio" aria-checked={value === icon.id} aria-label={icon.label} title={icon.label} onclick={() => (value = icon.id)}>
        <ItemLogo {loader} icon={icon.id} size={28} />
      </button>
    {/each}
  </div>
</fieldset>

<style>
  .icon-picker {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 12px 16px 16px;
    margin: 0;
    min-width: 0;
  }

  .icon-picker legend {
    padding: 0 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
    gap: 8px;
  }

  .icon-option {
    display: grid;
    place-items: center;
    min-height: 48px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-alt);
  }

  .icon-option:hover {
    border-color: var(--border-strong);
  }

  .icon-option.selected {
    border-color: var(--action);
    box-shadow: 0 0 0 1px var(--action);
    background: var(--surface-raised);
  }
</style>
