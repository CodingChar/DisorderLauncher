<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
    import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import IconPicker from '../lib/components/IconPicker.svelte'
  import Input from '../lib/components/Input.svelte'
  import PerformanceOptions from '../lib/components/PerformanceOptions.svelte'
  import Select from '../lib/components/Select.svelte'
  import { api } from '../lib/api'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { normalizeIconId } from '../lib/itemIcons'
  import { nav } from '../lib/stores/nav.svelte'
  import { isCustomVersion, versionLabel } from '../lib/text'
  import { sessions } from '../lib/stores/sessions.svelte'
  import VersionPicker from '../lib/components/VersionPicker.svelte'
  import type { Optimization } from '../lib/types'

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
  let icon = $state('')
  let optimization = $state<Optimization>('off')

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmDelete = $state(false)

  onMount(() => {
    if (!instance) return
    name = instance.name
    version = instance.minecraftVersion
    profileId = instance.profileId
    icon = normalizeIconId(instance.icon)
    optimization = instance.optimization ?? 'off'
    if (instance.memory.mode === 'fixed') {
      memoryMode = 'fixed'
      maxGb = String(Math.round((instance.memory.maxMb / 1024) * 10) / 10)
    }
    if (instance.java.mode === 'path') {
      javaMode = 'path'
      javaPath = instance.java.path
    }
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
        // Una version personalizada se queda como esta: no se cambia por otra de la lista oficial.
        ...(isCustomVersion(version) ? {} : { minecraftVersion: version }),
        profileId,
        icon,
        optimization,
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
      nav.go('instances')
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
      nav.go('instances')
    } catch (e) {
      error = (e as Error).message
    }
  }
</script>

<div class="page">
  <PageHeading eyebrow="CONFIGURAR INSTANCIA" title={instance?.name ?? 'Instancia'} description="Nombre, versión, perfil, memoria y Java de esta instancia." />

  {#if !instance}
    <p class="library-notice">No hay ninguna instancia seleccionada.</p>
  {:else}
    {#if busy}
      <p class="library-notice" role="status">Esta instancia se está usando. Podrás editarla cuando termine.</p>
    {/if}

    <section class="settings-panel panel">
      <form
        class="form-stack"
        onsubmit={(e) => {
          e.preventDefault()
          save()
        }}
      >
        <Input label="Nombre" bind:value={name} maxlength={64} />
        {#if isCustomVersion(version)}
          <p class="subtle">Versión personalizada: <strong>{versionLabel(version)}</strong>. Para usar otra, crea una instancia nueva.</p>
        {:else}
          <VersionPicker loader={instance.loader} bind:value={version} disabled={busy} autoSelect={false} />
        {/if}
        <IconPicker bind:value={icon} loader={instance.loader} disabled={busy} />
        <Select label="Perfil que juega" bind:value={profileId} options={profileOptions} disabled={busy} />

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

        {#if error}<p class="form-error" role="alert">{error}</p>{/if}
        {#if notice}<p class="form-ok" role="status">{notice}</p>{/if}

        <div class="form-actions">
          <Button variant="primary" type="submit" disabled={busy || saving || name.trim().length === 0}>
            {saving ? 'Guardando…' : 'Guardar cambios'}
          </Button>
          <Button variant="ghost" onclick={() => nav.go('instances')}>Volver a Mis instancias</Button>
        </div>
      </form>
    </section>

    <section class="settings-panel panel">
      <div class="section-title"><h3>Más acciones</h3></div>
      <div class="form-actions">
        <Button disabled={busy} onclick={duplicate}>Duplicar</Button>
        <Button disabled={busy} onclick={exportPack}>Exportar como modpack…</Button>
        <Button disabled={busy} onclick={() => (confirmDelete = true)}>Eliminar…</Button>
      </div>
    </section>

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
    gap: 20px;
    max-width: 800px;
  }
</style>
