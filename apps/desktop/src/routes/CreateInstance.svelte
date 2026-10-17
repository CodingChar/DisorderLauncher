<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import Input from '../lib/components/Input.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import Select from '../lib/components/Select.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { versions } from '../lib/stores/versions.svelte'
  import type { LoaderKind } from '../lib/types'

  let name = $state('Mi mundo')
  let version = $state('')
  let loader = $state<LoaderKind>('vanilla')
  let error = $state<string | null>(null)
  let busy = $state(false)
  let importing = $state(false)

  const loaderOptions = [
    { value: 'vanilla', label: 'Minecraft original (sin mods)' },
    { value: 'fabric', label: 'Fabric (permite instalar mods)' },
    { value: 'quilt', label: 'Quilt (compatible con mods de Fabric)' },
    { value: 'forge', label: 'Forge (mods de Forge)' },
    { value: 'neoforge', label: 'NeoForge (mods recientes, desde 1.20.2)' },
  ]
  const options = $derived(versions.list.map((v) => ({ value: v.id, label: v.id })))

  onMount(() => {
    if (versions.list.length === 0) versions.load()
  })

  // Preselecciona la version estable mas reciente cuando llega el catalogo.
  $effect(() => {
    if (!version && versions.list.length > 0) version = versions.list[0].id
  })

  async function importPack() {
    if (importing) return
    importing = true
    error = null
    try {
      const instance = await instances.importModpack()
      if (instance) nav.go('home')
    } catch (e) {
      error = (e as Error).message
    } finally {
      importing = false
    }
  }

  async function create() {
    if (busy) return
    busy = true
    error = null
    try {
      await instances.create(name, version, loader)
      nav.go('home')
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }
</script>

<div class="page">
  <header>
    <small>NUEVA INSTANCIA</small>
    <h1>Crea tu mundo</h1>
  </header>

  <Card>
    <form
      onsubmit={(e) => {
        e.preventDefault()
        create()
      }}
    >
      <Input label="Nombre" bind:value={name} maxlength={64} />
      <Select label="Tipo de instancia" bind:value={loader} options={loaderOptions} />

      {#if versions.loading}
        <p class="note">Buscando versiones de Minecraft…</p>
      {:else if versions.error}
        <div class="note-block" role="alert">
          <p>{versions.error}</p>
          <Button onclick={() => versions.load()}>Reintentar</Button>
        </div>
      {:else if options.length === 0}
        <div class="note-block">
          <p>No hay versiones disponibles por ahora. Comprueba tu conexión a internet.</p>
          <Button onclick={() => versions.load()}>Buscar de nuevo</Button>
        </div>
      {:else}
        <Select label="Versión de Minecraft" bind:value={version} {options} />
      {/if}

      {#if error}
        <p class="form-error" role="alert">{error}</p>
      {/if}

      <div class="row">
        <Button variant="primary" type="submit" disabled={busy || !version || name.trim().length === 0}>
          {busy ? 'Creando…' : 'Crear instancia'}
        </Button>
        <Button variant="ghost" onclick={() => nav.go('home')}>Cancelar</Button>
      </div>
    </form>
  </Card>

  <Card>
    <h3>¿Tienes un modpack?</h3>
    <p class="note">Importa un archivo <strong>.mrpack</strong> (Modrinth) y se crea la instancia con todo lo necesario.</p>
    {#if importing && instances.importPercent !== null}
      <ProgressBar percent={instances.importPercent} label="Importando modpack" />
    {/if}
    <div class="row">
      <Button disabled={importing || busy} onclick={importPack}>{importing ? 'Importando…' : 'Importar modpack…'}</Button>
    </div>
  </Card>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 520px;
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
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .row {
    display: flex;
    gap: 12px;
  }

  h3 {
    font-size: 18px;
    margin-bottom: 6px;
  }

  .note,
  .note-block p {
    margin: 0;
    color: var(--text-secondary);
  }

  .note-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 10px;
  }

  .form-error {
    margin: 0;
    font-weight: 600;
  }
</style>
