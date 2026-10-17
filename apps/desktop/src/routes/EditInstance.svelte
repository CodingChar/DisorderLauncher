<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import Select from '../lib/components/Select.svelte'
  import { api } from '../lib/api'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import { versions } from '../lib/stores/versions.svelte'

  const instance = $derived(instances.selected)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)

  // Copia de trabajo del formulario; se inicializa una vez al abrir la pantalla.
  let name = $state('')
  let version = $state('')
  let profileId = $state('')
  let memoryMode = $state<'auto' | 'fixed'>('auto')
  let maxGb = $state('4')
  let javaMode = $state<'auto' | 'path'>('auto')
  let javaPath = $state('')

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmDelete = $state(false)

  onMount(() => {
    if (versions.list.length === 0) versions.load()
    if (!instance) return
    name = instance.name
    version = instance.minecraftVersion
    profileId = instance.profileId
    if (instance.memory.mode === 'fixed') {
      memoryMode = 'fixed'
      maxGb = String(Math.round((instance.memory.maxMb / 1024) * 10) / 10)
    }
    if (instance.java.mode === 'path') {
      javaMode = 'path'
      javaPath = instance.java.path
    }
  })

  // La version actual siempre debe poder elegirse, aunque el catalogo no este disponible.
  const versionOptions = $derived.by(() => {
    const ids = new Set(versions.list.map((v) => v.id))
    if (version) ids.add(version)
    return [...ids].map((id) => ({ value: id, label: id }))
  })
  const profileOptions = $derived(accounts.list.map((a) => ({ value: a.id, label: a.label })))

  async function save() {
    if (!instance || saving) return
    error = null
    notice = null

    const gb = Number(maxGb.replace(',', '.'))
    if (memoryMode === 'fixed' && (!Number.isFinite(gb) || gb < 0.5 || gb > 64)) {
      error = 'Escribe una cantidad de memoria entre 0,5 y 64 GB.'
      return
    }

    saving = true
    try {
      await instances.update(instance.id, {
        name,
        minecraftVersion: version,
        profileId,
        memory:
          memoryMode === 'auto'
            ? { mode: 'auto' }
            : { mode: 'fixed', minMb: 512, maxMb: Math.round(gb * 1024) },
        java: javaMode === 'auto' ? { mode: 'auto' } : { mode: 'path', path: javaPath.trim() },
      })
      notice = 'Cambios guardados.'
    } catch (e) {
      error = (e as Error).message
    } finally {
      saving = false
    }
  }

  async function duplicate() {
    if (!instance) return
    try {
      await instances.duplicate(instance.id)
      nav.go('home')
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function exportPack() {
    if (!instance) return
    error = null
    notice = null
    try {
      const summary = await api.modpacksExport(instance.id)
      if (summary) {
        notice = `Modpack guardado: ${summary.remoteFiles} mods por enlace y ${summary.bundledFiles} archivos incluidos.`
      }
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function remove() {
    if (!instance) return
    confirmDelete = false
    try {
      await instances.remove(instance.id)
      nav.go('home')
    } catch (e) {
      error = (e as Error).message
    }
  }
</script>

<div class="page">
  <header>
    <small>EDITAR INSTANCIA</small>
    <h1>{instance?.name ?? 'Instancia'}</h1>
  </header>

  {#if !instance}
    <Card><p class="muted">No hay ninguna instancia seleccionada.</p></Card>
  {:else}
    {#if busy}
      <p class="banner" role="status">Esta instancia se está usando. Podrás editarla cuando termine.</p>
    {/if}

    <Card>
      <form
        onsubmit={(e) => {
          e.preventDefault()
          save()
        }}
      >
        <Input label="Nombre" bind:value={name} maxlength={64} />
        <Select label="Versión de Minecraft" bind:value={version} options={versionOptions} disabled={busy} />
        <Select label="Perfil que juega" bind:value={profileId} options={profileOptions} disabled={busy} />

        <fieldset>
          <legend>Memoria (RAM)</legend>
          <label class="radio"><input type="radio" bind:group={memoryMode} value="auto" /> Automática (recomendado)</label>
          <label class="radio"><input type="radio" bind:group={memoryMode} value="fixed" /> Elegir cantidad</label>
          {#if memoryMode === 'fixed'}
            <Input label="Máximo en GB" bind:value={maxGb} maxlength={5} hint="Más memoria no siempre es mejor. 4 GB suele bastar." />
          {/if}
        </fieldset>

        <fieldset>
          <legend>Java</legend>
          <label class="radio"><input type="radio" bind:group={javaMode} value="auto" /> Automático (recomendado)</label>
          <label class="radio"><input type="radio" bind:group={javaMode} value="path" /> Usar mi propio Java</label>
          {#if javaMode === 'path'}
            <Input label="Carpeta de Java" bind:value={javaPath} placeholder="C:\Program Files\Java\jdk-21" hint="La carpeta que contiene «bin»." />
          {/if}
        </fieldset>

        {#if error}<p class="error" role="alert">{error}</p>{/if}
        {#if notice}<p class="notice" role="status">{notice}</p>{/if}

        <div class="row">
          <Button variant="primary" type="submit" disabled={busy || saving || name.trim().length === 0}>
            {saving ? 'Guardando…' : 'Guardar cambios'}
          </Button>
          <Button variant="ghost" onclick={() => nav.go('home')}>Volver</Button>
        </div>
      </form>
    </Card>

    <Card>
      <h3>Más acciones</h3>
      <div class="row">
        <Button disabled={busy} onclick={duplicate}>Duplicar</Button>
        <Button disabled={busy} onclick={exportPack}>Exportar como modpack…</Button>
        <Button disabled={busy} onclick={() => (confirmDelete = true)}>Eliminar…</Button>
      </div>
    </Card>

    <ConfirmDialog
      open={confirmDelete}
      title="¿Eliminar esta instancia?"
      message="Se borrarán sus mundos, mods y ajustes. Esta acción no se puede deshacer."
      confirmLabel="Eliminar"
      onconfirm={remove}
      oncancel={() => (confirmDelete = false)}
    />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 560px;
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
    margin-bottom: 12px;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  fieldset {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 12px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  legend {
    padding: 0 6px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
  }

  .radio input {
    width: 20px;
    height: 20px;
    accent-color: var(--action);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }

  .banner,
  .muted {
    margin: 0;
    color: var(--text-secondary);
  }

  .error {
    margin: 0;
    font-weight: 600;
  }

  .notice {
    margin: 0;
    color: var(--text-secondary);
  }
</style>
