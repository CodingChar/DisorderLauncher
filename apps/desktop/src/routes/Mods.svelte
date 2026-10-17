<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { mods } from '../lib/stores/mods.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import type { ModEntry } from '../lib/types'

  const instance = $derived(instances.selected)
  const running = $derived(instance ? sessions.isBusy(instance.id) : false)
  const overview = $derived(mods.overview)

  let tab = $state<'installed' | 'search'>('installed')
  let query = $state('')
  let toRemove = $state<ModEntry | null>(null)

  onMount(() => {
    mods.reset()
    if (instance) mods.load(instance.id)
  })
  onDestroy(() => mods.reset())

  const installedProjects = $derived(new Set(overview?.mods.map((m) => m.source?.projectId).filter(Boolean)))

  function formatSize(bytes: number) {
    return bytes > 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`
  }

  function formatDownloads(n: number) {
    return n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)} M` : n >= 1000 ? `${Math.round(n / 1000)} mil` : String(n)
  }

  function updateFor(file: string) {
    const base = file.replace(/\.disabled$/, '')
    return mods.updates?.find((u) => u.file === base)
  }

  async function search() {
    if (instance) await mods.search(instance.id, query.trim())
  }

  function openSearch() {
    tab = 'search'
    if (instance && mods.results.length === 0) mods.search(instance.id, '')
  }
</script>

<div class="page">
  <header>
    <small>MODS</small>
    <h1>{instance?.name ?? 'Mods'}</h1>
  </header>

  {#if !instance}
    <Card><p class="muted">No hay ninguna instancia seleccionada.</p></Card>
  {:else if overview && !overview.supported}
    <Card>
      <h3>Esta instancia no admite mods</h3>
      <p class="muted">Para usar mods crea una instancia de tipo <strong>Fabric</strong>.</p>
      <div class="row">
        <Button variant="primary" onclick={() => nav.go('create')}>Crear instancia con Fabric</Button>
        <Button variant="ghost" onclick={() => nav.go('home')}>Volver</Button>
      </div>
    </Card>
  {:else}
    {#if running}
      <p class="banner" role="status">El juego está abierto. Ciérralo para cambiar los mods.</p>
    {/if}

    <div class="tabs" role="tablist" aria-label="Secciones de mods">
      <button role="tab" aria-selected={tab === 'installed'} class:active={tab === 'installed'} onclick={() => (tab = 'installed')}>
        Instalados{overview ? ` (${overview.mods.length})` : ''}
      </button>
      <button role="tab" aria-selected={tab === 'search'} class:active={tab === 'search'} onclick={openSearch}>Buscar mods</button>
      <span class="spacer"></span>
      <Button variant="ghost" onclick={() => nav.go('home')}>Volver</Button>
    </div>

    {#if mods.error}<p class="error" role="alert">{mods.error}</p>{/if}
    {#if mods.notice}<p class="notice" role="status">{mods.notice}</p>{/if}

    {#if overview && overview.missingDependencies.length > 0}
      <div class="warning" role="alert">
        {#each overview.missingDependencies as dep (dep.modName + dep.requires)}
          <p><strong>{dep.modName}</strong> necesita <strong>{dep.requires}</strong>. Instálalo desde «Buscar mods».</p>
        {/each}
      </div>
    {/if}

    {#if tab === 'installed'}
      <div class="toolbar">
        <Button disabled={running || mods.busy !== null} onclick={() => instance && mods.addLocal(instance.id)}>Agregar archivo…</Button>
        <Button disabled={mods.busy !== null || !overview?.mods.some((m) => m.source)} onclick={() => instance && mods.checkUpdates(instance.id)}>
          {mods.busy === 'updates' ? 'Buscando…' : 'Buscar actualizaciones'}
        </Button>
      </div>

      {#if mods.updates !== null && mods.updates.length === 0}
        <p class="muted">Todos tus mods están al día.</p>
      {/if}

      {#if mods.loading && !overview}
        <p class="muted">Cargando…</p>
      {:else if overview && overview.mods.length === 0}
        <Card>
          <h3>Aún no tienes mods</h3>
          <p class="muted">Busca e instala mods con un clic, o agrega un archivo que ya tengas.</p>
          <div class="row"><Button variant="primary" onclick={openSearch}>Buscar mods</Button></div>
        </Card>
      {:else if overview}
        <ul class="list">
          {#each overview.mods as mod (mod.fileName)}
            {@const update = updateFor(mod.fileName)}
            <li>
              <Card>
                <div class="mod">
                  <label class="switch" title={mod.enabled ? 'Desactivar' : 'Activar'}>
                    <input
                      type="checkbox"
                      role="switch"
                      checked={mod.enabled}
                      disabled={running || mods.busy !== null}
                      aria-label={`${mod.enabled ? 'Desactivar' : 'Activar'} ${mod.name}`}
                      onchange={(e) => instance && mods.setEnabled(instance.id, mod.fileName, e.currentTarget.checked)}
                    />
                    <span class="track" aria-hidden="true"></span>
                  </label>
                  <div class="info">
                    <strong class:off={!mod.enabled}>{mod.name}</strong>
                    <small>
                      {mod.version ?? ''}{mod.version ? ' · ' : ''}{formatSize(mod.size)}{mod.enabled ? '' : ' · Desactivado'}
                    </small>
                    {#if mod.description}<span class="desc">{mod.description}</span>{/if}
                  </div>
                  <div class="actions">
                    {#if update}
                      <Button variant="primary" disabled={running || mods.busy !== null} onclick={() => instance && mods.update(instance.id, update.file)}>
                        Actualizar a {update.latestVersion}
                      </Button>
                    {/if}
                    {#if mod.source?.hasPreviousVersion}
                      <Button variant="ghost" disabled={running || mods.busy !== null} onclick={() => instance && mods.rollback(instance.id, mod.fileName.replace(/\.disabled$/, ''))}>
                        Volver a la anterior
                      </Button>
                    {/if}
                    <Button variant="ghost" disabled={running || mods.busy !== null} onclick={() => (toRemove = mod)}>Quitar</Button>
                  </div>
                </div>
              </Card>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      <form
        class="search"
        onsubmit={(e) => {
          e.preventDefault()
          search()
        }}
      >
        <Input label="Buscar" bind:value={query} placeholder="Sodium, Lithium, minimapa…" hint="Solo se muestran mods compatibles con tu versión." />
        <Button type="submit" disabled={mods.busy === 'search'}>{mods.busy === 'search' ? 'Buscando…' : 'Buscar'}</Button>
      </form>

      <ul class="list">
        {#each mods.results as hit (hit.project_id)}
          <li>
            <Card>
              <div class="mod">
                <div class="info">
                  <strong>{hit.title}</strong>
                  <small>por {hit.author} · {formatDownloads(hit.downloads)} descargas</small>
                  <span class="desc">{hit.description}</span>
                </div>
                <div class="actions">
                  {#if installedProjects.has(hit.project_id)}
                    <span class="installed">Instalado</span>
                  {:else}
                    <Button variant="primary" disabled={running || mods.busy !== null} onclick={() => instance && mods.install(instance.id, hit)}>
                      {mods.busy === hit.project_id ? 'Instalando…' : 'Instalar'}
                    </Button>
                  {/if}
                </div>
              </div>
            </Card>
          </li>
        {:else}
          {#if mods.busy !== 'search'}
            <li><p class="muted">No se encontraron mods compatibles.</p></li>
          {/if}
        {/each}
      </ul>
    {/if}

    <ConfirmDialog
      open={toRemove !== null}
      title="¿Quitar este mod?"
      message={toRemove ? `Se eliminará «${toRemove.name}» de esta instancia.` : ''}
      confirmLabel="Quitar"
      onconfirm={() => {
        const target = toRemove
        toRemove = null
        if (target && instance) mods.remove(instance.id, target.fileName)
      }}
      oncancel={() => (toRemove = null)}
    />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 760px;
  }

  header small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.18em;
    margin-bottom: 5px;
  }

  h1 {
    font-size: 34px;
    line-height: 0.95;
    overflow-wrap: anywhere;
  }

  h3 {
    font-size: 18px;
    margin-bottom: 6px;
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid var(--border);
  }

  .tabs [role='tab'] {
    min-height: 44px;
    padding: 0 16px;
    background: transparent;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .tabs [role='tab'].active {
    color: var(--text-primary);
    border-bottom-color: var(--action);
  }

  .spacer {
    flex: 1;
  }

  .toolbar,
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .row {
    margin-top: 14px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .mod {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .info {
    flex: 1 1 260px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .info small {
    color: var(--text-secondary);
  }

  .desc {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 4px;
  }

  .off {
    color: var(--text-secondary);
    text-decoration: line-through;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .installed {
    padding: 8px 14px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
  }

  .switch {
    position: relative;
    display: inline-flex;
    width: 52px;
    height: 44px;
    align-items: center;
    flex: none;
    cursor: pointer;
  }

  .switch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    margin: 0;
    cursor: pointer;
  }

  .track {
    width: 44px;
    height: 24px;
    border-radius: 999px;
    background: var(--surface-raised);
    border: 1px solid var(--border-strong);
    position: relative;
    transition: background 0.18s;
  }

  .track::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--text-secondary);
    transition: transform 0.18s, background 0.18s;
  }

  .switch input:checked + .track {
    background: var(--action);
    border-color: var(--action);
  }

  .switch input:checked + .track::after {
    transform: translateX(20px);
    background: var(--action-text);
  }

  .switch input:focus-visible + .track {
    outline: 2px solid var(--action);
    outline-offset: 2px;
  }

  .search {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }

  .search :global(.field) {
    flex: 1;
  }

  .search :global(button) {
    margin-top: 22px;
  }

  .muted,
  .banner {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .error {
    margin: 0;
    font-weight: 600;
  }

  .notice {
    margin: 0;
    color: var(--text-secondary);
  }

  .warning {
    border: 1px dashed var(--text-primary);
    border-radius: var(--r-md);
    padding: 10px 14px;
  }

  .warning p {
    margin: 4px 0;
  }
</style>
