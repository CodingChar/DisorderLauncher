<script lang="ts">
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McTabs from '../lib/mc/McTabs.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import { CustomVersions } from '../lib/customVersions.svelte'
  import { LOADER_OPTIONS, PERFORMANCE_LEVELS, javaError, javaSelection, memoryError, memoryPolicy } from '../lib/instanceForm'
  import { findItemIcon } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { useVersionPicker } from '../lib/versionPicker.svelte'
  import type { LoaderKind, Optimization } from '../lib/types'
  import AdvancedOptions from './parts/AdvancedOptions.svelte'
  import CustomVersionScreen from './parts/CustomVersionScreen.svelte'
  import IconScreen from './parts/IconScreen.svelte'
  import VersionScreen from './parts/VersionScreen.svelte'

  const gui = useGui()

  type Sub = 'main' | 'version' | 'advanced' | 'icon'
  let sub = $state<Sub>('main')

  let name = $state('Mi mundo')
  let version = $state('')
  // «Oficial»: la lista de Mojang. «Personalizado»: versiones instaladas fuera del launcher (OptiFine, Forge...).
  let source = $state<'official' | 'custom'>('official')
  const custom = new CustomVersions()
  let loader = $state<LoaderKind>(nav.createLoader)
  let error = $state<string | null>(null)
  let busy = $state(false)
  let importing = $state(false)
  let icon = $state('')
  let optimization = $state<Optimization>('off')
  let memoryMode = $state<'auto' | 'fixed'>('auto')
  let maxGb = $state(4)
  let javaMode = $state<'auto' | 'path'>('auto')
  let javaPath = $state('')

  const picker = useVersionPicker({ loader: () => loader, value: () => version, setValue: (v) => (version = v) })

  // Las versiones instaladas fuera del launcher se buscan la primera vez que se abre esa pestana.
  $effect(() => {
    if (source === 'custom') custom.ensureLoaded()
  })

  // «Atras» (o Esc) en una subpantalla vuelve a la principal, no a Mis instancias.
  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  const iconLabel = $derived(icon ? (findItemIcon(icon)?.label ?? 'Por defecto') : 'Por defecto')
  const level = $derived(PERFORMANCE_LEVELS.find((l) => l.value === optimization) ?? PERFORMANCE_LEVELS[0])
  const chosenVersion = $derived(source === 'custom' ? custom.value : version)
  const canCreate = $derived(!busy && !importing && !!chosenVersion && name.trim().length > 0)

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
    if (busy || !canCreate) return
    const problem = memoryError(memoryMode, maxGb) ?? javaError(javaMode, javaPath)
    if (problem) {
      error = problem
      return
    }
    busy = true
    error = null
    try {
      const instance =
        source === 'custom'
          ? await instances.createCustom(name, custom.dir, custom.value, custom.copyData)
          : await instances.create(name, version, loader)
      // Los ajustes avanzados solo se guardan si se cambiaron; lo normal es dejarlos en automatico.
      if (memoryMode === 'fixed' || javaMode === 'path' || icon || optimization !== 'off') {
        await instances.update(instance.id, {
          ...(icon ? { icon } : {}),
          ...(optimization !== 'off' ? { optimization } : {}),
          ...(memoryMode === 'fixed' ? { memory: memoryPolicy('fixed', maxGb) } : {}),
          ...(javaMode === 'path' ? { java: javaSelection('path', javaPath) } : {}),
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

{#if sub === 'version' && source === 'official'}
  <VersionScreen {loader} value={version} entries={picker.entries} level={picker.level} disabled={busy} onchange={(id) => (version = id)} onlevel={(l) => (picker.level = l)} onback={() => (sub = 'main')} />
{:else if sub === 'version'}
  <CustomVersionScreen {custom} disabled={busy} onback={() => (sub = 'main')} />
{:else if sub === 'icon'}
  <IconScreen bind:value={icon} loader={source === 'custom' ? 'vanilla' : loader} disabled={busy} onback={() => (sub = 'main')} />
{:else if sub === 'advanced'}
  <AdvancedOptions bind:memoryMode bind:maxGb bind:javaMode bind:javaPath disabled={busy} onback={() => (sub = 'main')} />
{:else}
  <McScreen title="Crea tu mundo">
    <form
      class="mc-form"
      onsubmit={(e) => {
        e.preventDefault()
        create()
      }}
    >
      <McTabs
        tabs={[
          { id: 'official', label: 'Versiones oficiales' },
          { id: 'custom', label: 'Personalizado' },
        ]}
        bind:value={source}
        x={-155}
        y={22}
        w={310}
        disabled={busy || importing}
      />

      <McLabel x={-155} y={46} color="#a0a0a0" text="Nombre" />
      <McField x={-155} y={56} w={310} maxlength={64} bind:value={name} disabled={busy} placeholder="Mi mundo" label="Nombre de la instancia" />

      {#if source === 'official'}
        <McCycle x={-155} y={82} w={150} label="Tipo" options={LOADER_OPTIONS} bind:value={loader} disabled={busy} />
        <McButton x={5} y={82} w={150} label="Versión: {version || 'Elegir...'}" ariaLabel="Versión de Minecraft: {version || 'sin elegir'}. Pulsa para cambiar" disabled={busy} onclick={() => (sub = 'version')} />
      {:else}
        <McButton x={-155} y={82} w={310} label="Versión instalada: {custom.value || 'Elegir...'}" ariaLabel="Versión instalada: {custom.value || 'sin elegir'}. Pulsa para cambiar" disabled={busy} onclick={() => (sub = 'version')} />
      {/if}

      <McButton x={-155} y={106} w={150} label="Icono: {iconLabel}" disabled={busy} onclick={() => (sub = 'icon')} />
      <McCycle x={5} y={106} w={150} label="Rendimiento" options={PERFORMANCE_LEVELS} bind:value={optimization} disabled={busy} />

      <McButton x={-155} y={130} w={150} label="Opciones avanzadas..." disabled={busy} onclick={() => (sub = 'advanced')} />
      {#if importing && instances.importPercent !== null}
        <McProgress percent={instances.importPercent} label="Importando modpack" x={5} y={140} w={150} />
      {:else}
        <McButton x={5} y={130} w={150} label={importing ? 'Importando…' : 'Importar modpack...'} disabled={importing || busy} onclick={importPack} />
      {/if}

      <McText text={level.hint} x={-155} y={156} w={310} color="#808080" maxLines={2} />
      {#if error}<McText text={error} x={-155} y={gui.H - 50} w={310} color="#ff5555" maxLines={2} role="alert" />{/if}

      <McButton anchor="bottom" x={-155} y={-28} w={150} type="submit" label={busy ? 'Creando…' : 'Crear instancia'} disabled={!canCreate} />
      <McButton anchor="bottom" x={5} y={-28} w={150} label="Cancelar" onclick={() => nav.go('instances')} />
    </form>
  </McScreen>
{/if}
