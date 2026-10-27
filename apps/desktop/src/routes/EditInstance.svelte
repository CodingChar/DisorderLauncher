<script lang="ts">
  import { onMount } from 'svelte'
  import { goBack, pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import { bandWidth } from '../lib/mc/pos'
  import { api } from '../lib/api'
  import { PERFORMANCE_LEVELS, formFromInstance, javaSelection, memoryError, memoryPolicy } from '../lib/instanceForm'
  import { findItemIcon, normalizeIconId } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import { isCustomVersion, versionLabel } from '../lib/text'
  import { useVersionPicker } from '../lib/versionPicker.svelte'
  import type { Optimization } from '../lib/types'
  import AdvancedOptions from './parts/AdvancedOptions.svelte'
  import JavaTweaks from './parts/JavaTweaks.svelte'
  import IconScreen from './parts/IconScreen.svelte'
  import VersionScreen from './parts/VersionScreen.svelte'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const instance = $derived(instances.selected)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)

  type Sub = 'main' | 'version' | 'advanced' | 'jvm' | 'icon' | 'more'
  let sub = $state<Sub>('main')

  // Copia de trabajo del formulario; se inicializa una vez al abrir la pantalla.
  let name = $state('')
  let version = $state('')
  let memoryMode = $state<'auto' | 'fixed'>('auto')
  let maxGb = $state(4)
  let javaMode = $state<'auto' | 'path'>('auto')
  let javaPath = $state('')
  let icon = $state('')
  let optimization = $state<Optimization>('off')
  let jvmTweaks = $state<string[]>([])
  let jvmArgs = $state('')

  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmDelete = $state(false)

  // La edicion no cambia la version al cargar la lista (la instancia ya tiene la suya).
  const picker = useVersionPicker({ loader: () => instance?.loader ?? 'vanilla', value: () => version, setValue: (v) => (version = v), autoSelect: false })

  onMount(() => {
    if (!instance) return
    name = instance.name
    version = instance.minecraftVersion
    icon = normalizeIconId(instance.icon)
    optimization = instance.optimization ?? 'off'
    jvmTweaks = [...(instance.jvmTweaks ?? [])]
    jvmArgs = instance.jvmArgs ?? ''
    const form = formFromInstance(instance)
    memoryMode = form.memoryMode
    maxGb = form.maxGb
    javaMode = form.javaMode
    javaPath = form.javaPath
  })

  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  const custom = $derived(isCustomVersion(version))
  const iconLabel = $derived(icon ? (findItemIcon(icon)?.label ?? 'Por defecto') : 'Por defecto')
  const level = $derived(PERFORMANCE_LEVELS.find((l) => l.value === optimization) ?? PERFORMANCE_LEVELS[0])

  /** Aplica el nivel que mejor encaja con este equipo; la memoria automatica ya se ajusta sola a los mods. */
  async function recommend(): Promise<string> {
    if (!instance) return ''
    const r = await api.instancesRecommendation(instance.id)
    optimization = r.optimization
    const label = PERFORMANCE_LEVELS.find((l) => l.value === r.optimization)?.label ?? ''
    return `Nivel «${label}» elegido. ${r.reasons.join(' ')} Pulsa Guardar para conservarlo.`
  }

  async function save() {
    if (!instance || saving) return
    error = null
    notice = null

    const problem = memoryError(memoryMode, maxGb)
    if (problem) {
      error = problem
      return
    }

    saving = true
    try {
      await instances.update(instance.id, {
        name,
        // Una version personalizada se queda como esta: no se cambia por otra de la lista oficial.
        ...(custom ? {} : { minecraftVersion: version }),
        icon,
        optimization,
        jvmTweaks,
        jvmArgs,
        memory: memoryPolicy(memoryMode, maxGb),
        java: javaSelection(javaMode, javaPath),
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

{#if !instance}
  <McScreen title="Configurar instancia">
    <McText text="No hay ninguna instancia seleccionada." x={-155} y={90} w={310} align="center" color="#a0a0a0" />
    <McButton anchor="bottom" y={-28} label="Volver" onclick={goBack} />
  </McScreen>
{:else if sub === 'version'}
  <VersionScreen loader={instance.loader} value={version} entries={picker.entries} level={picker.level} disabled={busy} onchange={(id) => (version = id)} onlevel={(l) => (picker.level = l)} onback={() => (sub = 'main')} />
{:else if sub === 'icon'}
  <IconScreen bind:value={icon} loader={instance.loader} disabled={busy} onback={() => (sub = 'main')} />
{:else if sub === 'advanced'}
  <AdvancedOptions bind:memoryMode bind:maxGb bind:javaMode bind:javaPath disabled={busy} onback={() => (sub = 'main')} onjava={() => (sub = 'jvm')} onrecommend={recommend} />
{:else if sub === 'jvm'}
  <JavaTweaks instanceId={instance.id} level={optimization} bind:tweaks={jvmTweaks} bind:args={jvmArgs} disabled={busy} onback={() => (sub = 'advanced')} />
{:else if sub === 'more'}
  <McScreen title="Más acciones">
    <McButton y={28} label="Duplicar" disabled={busy} onclick={duplicate} />
    <McButton y={52} label="Exportar como modpack..." disabled={busy} onclick={exportPack} />
    <McButton y={76} label="Eliminar..." disabled={busy} onclick={() => (confirmDelete = true)} />
    {#if error}
      <McText text={error} x={-155} y={108} w={310} align="center" color="#ff5555" maxLines={3} role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={108} w={310} align="center" color="#55ff55" maxLines={3} role="status" />
    {/if}
    <McButton anchor="bottom" y={-28} label="Hecho" onclick={() => (sub = 'main')} />

    {#if confirmDelete}
      <McDialog
        title="¿Eliminar esta instancia?"
        message={`Se borrarán sus mundos, mods y ajustes de «${instance.name}». Esta acción no se puede deshacer.`}
        buttons={[
          { label: 'Cancelar', onclick: () => (confirmDelete = false) },
          { label: 'Eliminar', onclick: remove },
        ]}
        cancelIndex={0}
        onclose={() => (confirmDelete = false)}
      />
    {/if}
  </McScreen>
{:else}
  <McScreen title={instance.name}>
    <form
      class="mc-form"
      onsubmit={(e) => {
        e.preventDefault()
        save()
      }}
    >
      <McLabel x={-155} y={24} color="#a0a0a0" text="Nombre" />
      <McField x={-155} y={34} w={310} maxlength={64} bind:value={name} disabled={busy} placeholder="Nombre de la instancia" label="Nombre de la instancia" />

      <McButton
        x={-155}
        y={60}
        w={150}
        label="Versión: {versionLabel(version)}"
        ariaLabel={custom ? `Versión personalizada: ${versionLabel(version)}` : `Versión de Minecraft: ${version}. Pulsa para cambiar`}
        disabled={busy || custom}
        onclick={() => (sub = 'version')}
      />
      <McButton x={5} y={60} w={150} label="Icono: {iconLabel}" disabled={busy} onclick={() => (sub = 'icon')} />

      <McCycle x={-155} y={84} w={310} label="Rendimiento" options={PERFORMANCE_LEVELS} bind:value={optimization} disabled={busy} />

      <McButton x={-155} y={108} w={150} label="Opciones avanzadas..." disabled={busy} onclick={() => (sub = 'advanced')} />
      <McButton x={5} y={108} w={150} label="Más acciones..." onclick={() => (sub = 'more')} />

      <McText text={level.hint} x={-155} y={134} w={310} color="#808080" maxLines={2} />
      {#if custom}<McText text="Versión personalizada: para usar otra, crea una instancia nueva." x={-155} y={156} w={310} color="#808080" maxLines={2} />{/if}

      {#if busy}
        <McText text="Esta instancia se está usando. Podrás editarla cuando termine." x={-155} y={gui.H - 62} w={310} color="#ffaa00" role="status" />
      {/if}
      {#if error}
        <McText text={error} x={-155} y={gui.H - 50} w={310} color="#ff5555" maxLines={2} role="alert" />
      {:else if notice}
        <McText text={notice} x={-155} y={gui.H - 50} w={310} color="#55ff55" maxLines={2} role="status" />
      {/if}

      <McButton anchor="bottom" x={-155} y={-28} w={150} type="submit" label={saving ? 'Guardando…' : 'Guardar'} ariaLabel="Guardar cambios" disabled={busy || saving || name.trim().length === 0} />
      <McButton anchor="bottom" x={5} y={-28} w={150} label="Volver" onclick={goBack} />
    </form>
  </McScreen>
{/if}
