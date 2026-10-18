<script lang="ts">
  import Button from './Button.svelte'
  import Select from './Select.svelte'
  import { versionKindLabel, versions, type VersionKind } from '../stores/versions.svelte'
  import type { LoaderKind } from '../types'

  interface Props {
    loader: LoaderKind
    value: string
    disabled?: boolean
    /** Al cambiar de tipo de instancia, pasar a una version valida (no al editar una existente). */
    autoSelect?: boolean
  }

  let { loader, value = $bindable(), disabled = false, autoSelect = true }: Props = $props()

  // Estables siempre; el resto solo si la persona lo pide (la lista completa es muy larga).
  let show = $state<Record<VersionKind, boolean>>({ release: true, snapshot: false, old_beta: false, old_alpha: false })

  // Se pide la lista cada vez que cambia el tipo de instancia: cada loader soporta versiones distintas.
  $effect(() => {
    versions.load(loader)
  })

  const options = $derived.by(() => {
    const visible = versions.list.filter((v) => show[v.type])
    const items = visible.map((v) => ({ value: v.id, label: v.type === 'release' ? v.id : `${v.id} · ${versionKindLabel[v.type]}` }))
    // La version elegida siempre debe seguir en la lista (por ejemplo al editar una instancia antigua).
    if (value && !items.some((o) => o.value === value)) {
      const known = versions.list.find((v) => v.id === value)
      items.unshift({ value, label: known && known.type !== 'release' ? `${value} · ${versionKindLabel[known.type]}` : value })
    }
    return items
  })

  // Si la version elegida ya no esta (cambio de tipo de instancia), se pasa a la primera disponible.
  $effect(() => {
    if (!autoSelect || versions.loading || versions.loader !== loader) return
    const first = versions.list.find((v) => show[v.type])?.id
    if (first && (!value || !versions.list.some((v) => v.id === value))) value = first
  })

  const toggles: { kind: VersionKind; label: string }[] = [
    { kind: 'snapshot', label: 'Snapshots' },
    { kind: 'old_beta', label: 'Betas antiguas' },
    { kind: 'old_alpha', label: 'Alphas antiguas' },
  ]
</script>

{#if versions.loading && versions.list.length === 0}
  <p class="subtle" role="status">Buscando versiones de Minecraft…</p>
{:else if versions.error && versions.list.length === 0}
  <div class="note-block" role="alert">
    <p class="form-error">{versions.error}</p>
    <Button onclick={() => versions.load(loader)}>Reintentar</Button>
  </div>
{:else if versions.list.length === 0}
  <div class="note-block">
    <p class="subtle">No hay versiones disponibles para este tipo de instancia. Conéctate a internet para ver la lista completa.</p>
    <Button onclick={() => versions.load(loader)}>Buscar de nuevo</Button>
  </div>
{:else}
  <Select label="Versión de Minecraft" bind:value {options} {disabled} />
  <div class="version-filters" role="group" aria-label="Qué versiones mostrar">
    {#each toggles as t (t.kind)}
      <label class="check-row"><input type="checkbox" bind:checked={show[t.kind]} {disabled} /> {t.label}</label>
    {/each}
  </div>
  {#if versions.source === 'cached'}
    <p class="subtle" role="status">Sin conexión: se muestra la última lista descargada.</p>
  {:else if versions.source === 'installed'}
    <p class="subtle" role="status">Sin conexión: solo aparecen las versiones que ya tienes instaladas.</p>
  {/if}
{/if}

<style>
  .version-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
  }

  .note-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }
</style>
