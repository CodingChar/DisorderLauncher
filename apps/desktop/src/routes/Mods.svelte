<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { goBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McTabs from '../lib/mc/McTabs.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { defaultIconFor, findItemIcon } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { mods } from '../lib/stores/mods.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import type { ModEntry, ProjectHit } from '../lib/types'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const instance = $derived(instances.selected)
  const running = $derived(instance ? sessions.isBusy(instance.id) : false)
  const overview = $derived(mods.overview)
  const locked = $derived(running || mods.busy !== null)

  let tab = $state<'installed' | 'search'>('installed')
  let query = $state('')
  let toRemove = $state<ModEntry | null>(null)
  let modId = $state<string | null>(null)
  let hitId = $state<string | null>(null)

  onMount(() => {
    mods.reset()
    if (instance) mods.load(instance.id)
  })
  onDestroy(() => mods.reset())

  const installedProjects = $derived(new Set(overview?.mods.map((m) => m.source?.projectId).filter(Boolean)))
  const selectedMod = $derived(overview?.mods.find((m) => m.fileName === modId) ?? null)
  const selectedHit = $derived(mods.results.find((h) => h.project_id === hitId) ?? null)
  const selectedUpdate = $derived(selectedMod ? updateFor(selectedMod.fileName) : undefined)

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

  const busyLabel = $derived(
    mods.busy === 'search' ? 'Buscando mods' : mods.busy === 'optimize' ? 'Instalando mejoras de rendimiento' : mods.busy === 'updates' ? 'Buscando actualizaciones' : mods.busy === 'add' ? 'Agregando el archivo' : 'Trabajando',
  )

  // Mensajes bajo la lista: lo mas importante primero (maximo dos lineas).
  const messages = $derived.by(() => {
    const out: { text: string; color: string; role?: 'alert' | 'status' }[] = []
    if (mods.error) out.push({ text: mods.error, color: '#ff5555', role: 'alert' })
    if (mods.notice) out.push({ text: mods.notice, color: '#55ff55', role: 'status' })
    if (running) out.push({ text: 'El juego está abierto. Cierra el juego para cambiar mods.', color: '#ffaa00', role: 'status' })
    for (const dep of overview?.missingDependencies ?? []) out.push({ text: `${dep.modName} necesita ${dep.requires}. Instálalo desde «Buscar mods».`, color: '#ffaa00', role: 'alert' })
    if (tab === 'installed' && mods.updates !== null && mods.updates.length === 0) out.push({ text: 'Todos tus mods están al día.', color: '#808080', role: 'status' })
    return out.slice(0, 2)
  })

  const modIcon = $derived(instance ? defaultIconFor(instance.loader) : undefined)
  const genericIcon = findItemIcon('crafting-table')

  function modRow(m: ModEntry): RowSpec {
    const update = updateFor(m.fileName)
    return {
      icon: modIcon ? { src: modIcon.src } : undefined,
      lines: [
        { text: m.name, color: m.enabled ? '#ffffff' : '#808080' },
        { text: [m.version, formatSize(m.size), m.enabled ? '' : 'Desactivado'].filter(Boolean).join(' · ') },
        update ? { text: `Actualización disponible: ${update.latestVersion}`, color: '#ffaa00' } : { text: m.description ?? '' },
      ],
    }
  }

  function hitRow(h: ProjectHit): RowSpec {
    const installed = installedProjects.has(h.project_id)
    return {
      icon: genericIcon ? { src: genericIcon.src } : undefined,
      lines: [{ text: h.title }, { text: `por ${h.author} · ${formatDownloads(h.downloads)} descargas` }, installed ? { text: 'Instalado', color: '#55ff55' } : { text: h.description }],
    }
  }

  const tabs = $derived([
    { id: 'installed' as const, label: `Mis mods${overview ? ` (${overview.mods.length})` : ''}` },
    { id: 'search' as const, label: 'Buscar mods' },
  ])
</script>

<McScreen title={instance?.name ?? 'Mods'}>
  {#if !instance}
    <McText text="No hay ninguna instancia seleccionada." x={-155} y={90} w={310} align="center" color="#a0a0a0" />
    <McButton anchor="bottom" y={-28} label="Volver" onclick={() => goBack()} />
  {:else if overview && !overview.supported}
    <McText text="La gestión de mods no está disponible aquí" x={-155} y={50} w={310} align="center" />
    <McText
      text="Las instancias originales no llevan mods. Para instalarlos y actualizarlos desde el launcher crea una instancia con Fabric, Quilt, Forge o NeoForge."
      x={-155}
      y={70}
      w={310}
      align="center"
      color="#a0a0a0"
      maxLines={5}
    />
    <McButton y={140} label="Crear instancia con mods" onclick={() => nav.create('fabric')} />
    <McButton anchor="bottom" y={-28} label="Volver" onclick={() => goBack()} />
  {:else}
    <McTabs {tabs} bind:value={tab} x={-155} y={20} w={310} onchange={(id) => id === 'search' && openSearch()} />

    {#if tab === 'installed'}
      <McList
        items={overview?.mods ?? []}
        getId={(m) => m.fileName}
        row={modRow}
        bind:selectedId={modId}
        x={-(bw >> 1)}
        y={44}
        w={bw}
        h={gui.H - 134}
        loading={mods.loading && !overview}
        empty="Aún no tienes mods. Busca e instala mods con un clic, o agrega un archivo que ya tengas."
        ariaLabel="Mods instalados"
      />
      {#if mods.busy !== null}
        <McProgress percent={null} label={busyLabel} x={-155} w={310} y={gui.H - 68} />
      {:else}
        {#each messages as m, i (m.text)}
          <McText text={m.text} x={-155} y={gui.H - 84 + i * 10} w={310} align="center" color={m.color} role={m.role} maxLines={1} />
        {/each}
      {/if}

      <McButton
        anchor="bottom"
        x={-150}
        y={-52}
        w={72}
        label={selectedMod && !selectedMod.enabled ? 'Activar' : 'Desactivar'}
        ariaLabel={selectedMod ? `${selectedMod.enabled ? 'Desactivar' : 'Activar'} ${selectedMod.name}` : 'Activar o desactivar el mod'}
        disabled={locked || !selectedMod}
        onclick={() => selectedMod && mods.setEnabled(instance.id, selectedMod.fileName, !selectedMod.enabled)}
      />
      <McButton anchor="bottom" x={-74} y={-52} w={72} label="Quitar..." ariaLabel="Quitar este mod" disabled={locked || !selectedMod} onclick={() => (toRemove = selectedMod)} />
      <McButton anchor="bottom" x={2} y={-52} w={72} label="Actualizar" ariaLabel={selectedUpdate ? `Actualizar a ${selectedUpdate.latestVersion}` : 'Actualizar este mod'} disabled={locked || !selectedUpdate} onclick={() => selectedUpdate && mods.update(instance.id, selectedUpdate.file)} />
      <McButton
        anchor="bottom"
        x={78}
        y={-52}
        w={72}
        label="Revertir"
        ariaLabel="Volver a la versión anterior del mod"
        disabled={locked || !selectedMod?.source?.hasPreviousVersion}
        onclick={() => selectedMod && mods.rollback(instance.id, selectedMod.fileName.replace(/\.disabled$/, ''))}
      />
      <McButton anchor="bottom" x={-150} y={-28} w={72} label="Agregar..." ariaLabel="Agregar un archivo de mod" disabled={locked} onclick={() => mods.addLocal(instance.id)} />
      <McButton anchor="bottom" x={-74} y={-28} w={72} label="Optimizar" ariaLabel="Mejorar rendimiento" disabled={locked} onclick={() => mods.optimize(instance.id)} />
      <McButton anchor="bottom" x={2} y={-28} w={72} label="Comprobar" ariaLabel="Buscar actualizaciones" disabled={mods.busy !== null || !overview?.mods.some((m) => m.source)} onclick={() => mods.checkUpdates(instance.id)} />
      <McButton anchor="bottom" x={78} y={-28} w={72} label="Volver" onclick={() => goBack()} />
    {:else}
      <form
        class="mc-form"
        onsubmit={(e) => {
          e.preventDefault()
          search()
        }}
      >
        <McField x={-155} y={44} w={228} maxlength={60} bind:value={query} placeholder="Sodium, Lithium, minimapa…" label="Buscar mods" />
        <McButton x={77} y={44} w={78} type="submit" label={mods.busy === 'search' ? 'Buscando…' : 'Buscar'} disabled={mods.busy === 'search'} />
      </form>
      <McText text="Solo se muestran mods compatibles con tu versión." x={-155} y={68} w={310} color="#808080" maxLines={1} />
      <McList
        items={mods.results}
        getId={(h) => h.project_id}
        row={hitRow}
        bind:selectedId={hitId}
        x={-(bw >> 1)}
        y={80}
        w={bw}
        h={gui.H - 142}
        loading={mods.busy === 'search'}
        loadingText="Buscando..."
        empty="No se encontraron mods compatibles."
        ariaLabel="Resultados de la búsqueda"
      />
      {#if mods.busy !== null && mods.busy !== 'search'}
        <McProgress percent={null} label="Instalando" x={-155} w={310} y={gui.H - 42} />
      {:else}
        {#each messages as m, i (m.text)}
          <McText text={m.text} x={-155} y={gui.H - 52 + i * 10} w={310} align="center" color={m.color} role={m.role} maxLines={1} />
        {/each}
      {/if}
      <McButton
        anchor="bottom"
        x={-155}
        y={-28}
        w={150}
        label={selectedHit && mods.busy === selectedHit.project_id ? 'Instalando…' : 'Instalar'}
        ariaLabel={selectedHit ? `Instalar ${selectedHit.title}` : 'Instalar mod'}
        disabled={locked || !selectedHit || installedProjects.has(selectedHit.project_id)}
        onclick={() => selectedHit && mods.install(instance.id, selectedHit)}
      />
      <McButton anchor="bottom" x={5} y={-28} w={150} label="Volver" onclick={() => goBack()} />
    {/if}

    {#if toRemove}
      <McDialog
        title="¿Quitar este mod?"
        message={`Se eliminará «${toRemove.name}» de esta instancia.`}
        buttons={[
          { label: 'Cancelar', onclick: () => (toRemove = null) },
          {
            label: 'Quitar',
            onclick: () => {
              const target = toRemove
              toRemove = null
              if (target && instance) mods.remove(instance.id, target.fileName)
            },
          },
        ]}
        cancelIndex={0}
        onclose={() => (toRemove = null)}
      />
    {/if}
  {/if}
</McScreen>
