<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import Card from '../lib/components/Card.svelte'
  import IconPicker from '../lib/components/IconPicker.svelte'
  import Input from '../lib/components/Input.svelte'
  import PerformanceOptions from '../lib/components/PerformanceOptions.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import Select from '../lib/components/Select.svelte'
  import VersionPicker from '../lib/components/VersionPicker.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import type { LoaderKind, Optimization } from '../lib/types'

  let name = $state('Mi mundo')
  let version = $state('')
  let loader = $state<LoaderKind>(nav.createLoader)
  let error = $state<string | null>(null)
  let busy = $state(false)
  let importing = $state(false)
  let icon = $state('')
  let optimization = $state<Optimization>('off')
  let memoryMode = $state<'auto' | 'fixed'>('auto')
  let maxGb = $state('4')
  let javaMode = $state<'auto' | 'path'>('auto')
  let javaPath = $state('')

  const loaderOptions = [
    { value: 'vanilla', label: 'Minecraft original (sin mods)' },
    { value: 'fabric', label: 'Fabric (permite instalar mods)' },
    { value: 'quilt', label: 'Quilt (compatible con mods de Fabric)' },
    { value: 'forge', label: 'Forge (mods de Forge)' },
    { value: 'neoforge', label: 'NeoForge (mods recientes, desde 1.20.2)' },
  ]

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
    const gb = Number(maxGb.replace(',', '.'))
    if (memoryMode === 'fixed' && (!Number.isFinite(gb) || gb < 0.5 || gb > 64)) {
      error = 'Escribe una cantidad de memoria entre 0,5 y 64 GB.'
      return
    }
    if (javaMode === 'path' && javaPath.trim().length === 0) {
      error = 'Indica la carpeta de Java o elige «Automático».'
      return
    }
    busy = true
    error = null
    try {
      const instance = await instances.create(name, version, loader)
      // Los ajustes avanzados solo se guardan si se cambiaron; lo normal es dejarlos en automatico.
      if (memoryMode === 'fixed' || javaMode === 'path' || icon || optimization !== 'off') {
        await instances.update(instance.id, {
          ...(icon ? { icon } : {}),
          ...(optimization !== 'off' ? { optimization } : {}),
          ...(memoryMode === 'fixed' ? { memory: { mode: 'fixed' as const, minMb: 512, maxMb: Math.round(gb * 1024) } } : {}),
          ...(javaMode === 'path' ? { java: { mode: 'path' as const, path: javaPath.trim() } } : {}),
        })
      }
      nav.go('home')
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }
</script>

<div class="page">
  <PageHeading eyebrow="NUEVA INSTANCIA" title="Crea tu mundo" description="Elige un nombre, el tipo de instancia y la versión de Minecraft." />

  <section class="settings-panel panel">
    <form
      class="form-stack"
      onsubmit={(e) => {
        e.preventDefault()
        create()
      }}
    >
      <Input label="Nombre" bind:value={name} maxlength={64} />
      <Select label="Tipo de instancia" bind:value={loader} options={loaderOptions} />
      <IconPicker bind:value={icon} {loader} disabled={busy} />

      <VersionPicker {loader} bind:value={version} disabled={busy} />

      <details class="settings-block advanced">
        <summary>Opciones avanzadas</summary>
        <p class="subtle">Puedes dejarlas como están: no hace falta tocar nada para jugar. Cámbialas solo si sabes lo que necesita tu instancia.</p>

        <PerformanceOptions bind:value={optimization} disabled={busy} />

        <fieldset class="group">
          <legend>Memoria (RAM)</legend>
          <label class="check-row"><input type="radio" bind:group={memoryMode} value="auto" /> Automática (recomendado)</label>
          <label class="check-row"><input type="radio" bind:group={memoryMode} value="fixed" /> Elegir cantidad</label>
          {#if memoryMode === 'fixed'}
            <Input label="Máximo en GB" bind:value={maxGb} maxlength={5} hint="Más memoria no siempre es mejor. 4 GB suele bastar." />
          {/if}
        </fieldset>

        <fieldset class="group">
          <legend>Java</legend>
          <label class="check-row"><input type="radio" bind:group={javaMode} value="auto" /> Automático (recomendado)</label>
          <label class="check-row"><input type="radio" bind:group={javaMode} value="path" /> Usar mi propio Java</label>
          {#if javaMode === 'path'}
            <Input label="Carpeta de Java" bind:value={javaPath} placeholder="C:\Program Files\Java\jdk-21" hint="La carpeta que contiene «bin»." />
          {/if}
        </fieldset>
      </details>

      {#if error}
        <p class="form-error" role="alert">{error}</p>
      {/if}

      <div class="form-actions">
        <Button variant="primary" type="submit" disabled={busy || !version || name.trim().length === 0}>
          {busy ? 'Creando…' : 'Crear instancia'}
        </Button>
        <Button variant="ghost" onclick={() => nav.go('instances')}>Cancelar</Button>
      </div>
    </form>
  </section>

  <section class="settings-panel panel">
    <div class="section-title">
      <h3>¿Tienes un modpack?</h3>
    </div>
    <p class="subtle">Importa un archivo <strong>.mrpack</strong> (Modrinth) y se crea la instancia con todo lo necesario.</p>
    {#if importing && instances.importPercent !== null}
      <ProgressBar percent={instances.importPercent} label="Importando modpack" />
    {/if}
    <div class="form-actions pack-actions">
      <Button disabled={importing || busy} onclick={importPack}>{importing ? 'Importando…' : 'Importar modpack…'}</Button>
    </div>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 800px;
  }


  .pack-actions {
    margin-top: 16px;
  }
</style>
