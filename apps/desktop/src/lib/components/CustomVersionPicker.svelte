<script lang="ts">
  import { onMount } from 'svelte'
  import Button from './Button.svelte'
  import { api } from '../api'
  import { externalKindName } from '../text'
  import type { ExternalScan } from '../types'

  interface Props {
    /** Id (carpeta) de la version elegida; vacio si no hay ninguna. */
    value: string
    /** Carpeta de Minecraft revisada; `null` = la habitual de este equipo. */
    dir: string | null
    copyData: boolean
    disabled?: boolean
  }

  let { value = $bindable(), dir = $bindable(), copyData = $bindable(), disabled = false }: Props = $props()

  let scan = $state<ExternalScan | null>(null)
  let loading = $state(true)
  let error = $state<string | null>(null)

  function apply(result: ExternalScan, picked: boolean) {
    scan = result
    // Con la carpeta habitual se manda `null`; con una elegida a mano, su ruta (el backend la comprueba).
    dir = picked ? result.dir : null
    if (!result.versions.some((v) => v.id === value)) value = result.versions[0]?.id ?? ''
  }

  async function load() {
    loading = true
    error = null
    try {
      apply(await api.externalVersionsScan(null), false)
    } catch (e) {
      error = (e as Error).message
    } finally {
      loading = false
    }
  }

  async function pickFolder() {
    error = null
    try {
      const result = await api.externalVersionsPickDir()
      if (result) apply(result, true)
    } catch (e) {
      error = (e as Error).message
    }
  }

  onMount(load)
</script>

<div class="custom-picker">
  {#if loading}
    <p class="subtle" role="status">Buscando versiones instaladas fuera del launcher…</p>
  {:else}
    {#if error}<p class="form-error" role="alert">{error}</p>{/if}

    {#if scan && scan.versions.length > 0}
      <fieldset class="versions" {disabled}>
        <legend>Versiones encontradas</legend>
        {#each scan.versions as version (version.id)}
          <label class="version-row" class:selected={value === version.id}>
            <input type="radio" name="custom-version" bind:group={value} value={version.id} />
            <span class="version-name">
              <strong>{externalKindName(version.kind)}</strong> · {version.id}
              <small>
                {version.baseVersion ? `Minecraft ${version.baseVersion}` : 'Versión independiente'}{version.imported ? ' · ya usada antes' : ''}
              </small>
            </span>
          </label>
        {/each}
      </fieldset>
      <label class="check-row">
        <input type="checkbox" bind:checked={copyData} {disabled} />
        <span>Copiar también mis mundos, mods y ajustes de esa carpeta<small>Se hace una copia: tu carpeta original no se toca. Puede tardar si tienes muchos mundos.</small></span>
      </label>
    {:else if scan && scan.found}
      <p class="subtle">No hay versiones de OptiFine, Forge, Fabric u otras personalizadas en esta carpeta.</p>
    {:else}
      <p class="subtle">No encontramos una carpeta de Minecraft en este equipo. Si la tienes en otro lugar, búscala.</p>
    {/if}

    <div class="folder-row">
      {#if scan?.found}<span class="subtle folder" title={scan.dir}>Carpeta: {scan.dir}</span>{/if}
      <Button variant="ghost" {disabled} onclick={pickFolder}>Buscar en otra carpeta…</Button>
    </div>
  {/if}
</div>

<style>
  .custom-picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .versions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 12px 16px 16px;
    margin: 0;
    min-width: 0;
  }

  .versions legend {
    padding: 0 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .version-row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 48px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--r-sm);
    background: var(--surface-alt);
    cursor: pointer;
  }

  .version-row:hover {
    border-color: var(--border-strong);
  }

  .version-row.selected {
    border-color: var(--action);
    box-shadow: 0 0 0 1px var(--action);
    background: var(--surface-raised);
  }

  .version-name {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .version-name small {
    color: var(--text-secondary);
  }

  .folder-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 16px;
  }

  .folder {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
