<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { goBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
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
  import { categoryLabel, compatibilityOptions, environmentLabel, formatDownloads, loaderLabel, sideOptions, sinceLabel, sortLabels, type SideFilter } from '../lib/modCatalog'
  import type { Compatibility, ContentHit, ModEntry, SortKey } from '../lib/types'

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

  // Filtros en edicion (se aplican con «Aplicar»); los que ya se aplicaron viven en `mods.filters`.
  let filtersOpen = $state(false)
  let draftCategories = $state<string[]>([])
  let draftSide = $state<SideFilter>('any')
  let draftCompat = $state<Compatibility>('instance')
  let draftProvider = $state<string | null>(null)
  let categoryCursor = $state<string | null>(null)

  onMount(() => {
    if (instance) mods.open(instance.id)
    else mods.reset()
    void mods.loadCatalog()
  })
  onDestroy(() => mods.reset())

  const hitKey = (h: { provider: string; projectId: string }) => `${h.provider}:${h.projectId}`
  const installedProjects = $derived(new Set(overview?.mods.filter((m) => m.source).map((m) => hitKey(m.source!))))
  const selectedMod = $derived(overview?.mods.find((m) => m.fileName === modId) ?? null)
  const selectedHit = $derived(mods.results.find((h) => hitKey(h) === hitId) ?? null)

  const providers = $derived(mods.catalog?.providers ?? [])
  const activeProvider = $derived(providers.find((p) => p.id === mods.filters.provider) ?? providers[0] ?? null)
  const sortOptions = $derived((activeProvider?.sorts ?? (Object.keys(sortLabels) as SortKey[])).map((value) => ({ value, label: sortLabels[value] })))
  const draftProviderInfo = $derived(providers.find((p) => p.id === draftProvider) ?? providers[0] ?? null)
  const sideFilterAvailable = $derived(draftProviderInfo?.sideFilter ?? true)
  const categoryChoices = $derived(mods.catalog?.categories ?? [])
  const activeFilterCount = $derived(mods.filters.categories.length + (mods.filters.side ? 1 : 0) + (mods.filters.compatibility !== 'instance' ? 1 : 0))
  const totalText = $derived(
    mods.searched ? `${mods.total.toLocaleString('es')} ${mods.total === 1 ? 'resultado' : 'resultados'}${mods.results.length < mods.total ? ` · mostrando ${mods.results.length}` : ''}` : '',
  )
  const selectedUpdate = $derived(selectedMod ? updateFor(selectedMod.fileName) : undefined)

  function formatSize(bytes: number) {
    return bytes > 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`
  }

  function updateFor(file: string) {
    const base = file.replace(/\.disabled$/, '')
    return mods.updates?.find((u) => u.file === base)
  }

  async function search(patch: Partial<typeof mods.filters> = {}) {
    if (!instance) return
    hitId = null
    await mods.search(instance.id, { ...mods.filters, ...patch, text: query.trim() })
  }

  function openSearch() {
    tab = 'search'
    if (instance && !mods.searched) void search()
  }

  function openFilters() {
    draftCategories = [...mods.filters.categories]
    draftSide = mods.filters.side ?? 'any'
    draftCompat = mods.filters.compatibility
    draftProvider = mods.filters.provider ?? null
    filtersOpen = true
  }

  function applyFilters() {
    filtersOpen = false
    void search({ categories: [...draftCategories], side: draftSide === 'any' || !sideFilterAvailable ? null : draftSide, compatibility: draftCompat, provider: draftProvider })
  }

  function clearDraft() {
    draftCategories = []
    draftSide = 'any'
    draftCompat = 'instance'
  }

  function toggleCategory(id: string) {
    draftCategories = draftCategories.includes(id) ? draftCategories.filter((c) => c !== id) : [...draftCategories, id]
  }

  function categoryRow(id: string): RowSpec {
    const on = draftCategories.includes(id)
    return { lines: [{ text: `${on ? '[x]' : '[ ]'} ${categoryLabel(id)}`, color: on ? '#55ff55' : '#ffffff' }] }
  }

  const busyLabel = $derived(
    mods.busy === 'search' ? 'Buscando mods' : mods.busy === 'optimize' ? 'Instalando mejoras de rendimiento' : mods.busy === 'updates' ? 'Buscando actualizaciones' : mods.busy === 'add' ? 'Agregando el archivo y sus dependencias' : mods.busy === 'deps' ? 'Instalando las dependencias que faltan' : 'Trabajando',
  )

  // Mensajes bajo la lista: lo mas importante primero (maximo dos lineas).
  const messages = $derived.by(() => {
    const out: { text: string; color: string; role?: 'alert' | 'status' }[] = []
    if (mods.error) out.push({ text: mods.error, color: '#ff5555', role: 'alert' })
    if (mods.notice) out.push({ text: mods.notice, color: '#55ff55', role: 'status' })
    for (const c of overview?.conflicts ?? []) out.push({ text: `${c.first} y ${c.second} no se llevan bien: ${c.reason}. Desactiva uno de los dos.`, color: '#ff5555', role: 'alert' })
    if (running) out.push({ text: 'El juego está abierto. Cierra el juego para cambiar mods.', color: '#ffaa00', role: 'status' })
    for (const dep of overview?.missingDependencies ?? []) out.push({ text: `${dep.modName} necesita ${dep.requires}${mods.busy === 'deps' ? ': se está instalando.' : ' y no se encontró solo. Búscalo en «Buscar mods».'}`, color: '#ffaa00', role: 'alert' })
    if (tab === 'installed' && mods.updates !== null && mods.updates.length === 0) out.push({ text: 'Todos tus mods están al día.', color: '#808080', role: 'status' })
    // Caben dos lineas: lo que sobra se junta en la segunda (con «[+]» para leerlo entero) en vez de perderse.
    if (out.length <= 2) return out
    return [out[0], { ...out[1], text: out.slice(1).map((m) => m.text).join(' ') }]
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

  function hitRow(h: ContentHit): RowSpec {
    const installed = installedProjects.has(hitKey(h))
    return {
      icon: genericIcon ? { src: genericIcon.src } : undefined,
      lines: [
        { text: installed ? `${h.title} · Instalado` : h.title, color: installed ? '#55ff55' : '#ffffff' },
        { text: `por ${h.author} · ${formatDownloads(h.downloads)} descargas` },
        { text: h.summary },
      ],
    }
  }

  // Detalle del resultado elegido (lo que no cabe en su fila): categorias, loaders, entorno y novedad.
  const hitDetails = $derived.by(() => {
    const h = selectedHit
    if (!h) return []
    const since = sinceLabel(h.updatedAt)
    const first = [h.categories.map(categoryLabel).join(', '), h.loaders.map(loaderLabel).join(', ')].filter(Boolean).join(' · ')
    const second = [environmentLabel(h), h.follows > 0 ? `${formatDownloads(h.follows)} seguidores` : null, h.latestVersion ? `Minecraft ${h.latestVersion}` : null, since ? `actualizado ${since}` : null].filter(Boolean).join(' · ')
    return [first, second].filter(Boolean)
  })

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
          filtersOpen = false
          search()
        }}
      >
        <McField x={-155} y={44} w={228} maxlength={60} bind:value={query} placeholder="Sodium, Lithium, minimapa…" label="Buscar mods" />
        <McButton x={77} y={44} w={78} type="submit" label={mods.busy === 'search' ? 'Buscando…' : 'Buscar'} disabled={mods.busy === 'search'} />
      </form>
      <McCycle
        x={-155}
        y={68}
        w={152}
        label="Orden"
        options={sortOptions}
        value={mods.filters.sort}
        onchange={(sort) => {
          filtersOpen = false
          search({ sort })
        }}
      />
      <McButton
        x={3}
        y={68}
        w={152}
        label={filtersOpen ? 'Cerrar filtros' : activeFilterCount > 0 ? `Filtros (${activeFilterCount})...` : 'Filtros...'}
        ariaLabel="Filtros de búsqueda"
        onclick={() => (filtersOpen ? (filtersOpen = false) : openFilters())}
      />

      {#if filtersOpen}
        <McCycle x={-155} y={92} w={152} label="Entorno" options={sideOptions} bind:value={draftSide} disabled={!sideFilterAvailable} />
        <McCycle x={3} y={92} w={152} label="Para" options={compatibilityOptions} bind:value={draftCompat} />
        {#if providers.length > 1}
          <McCycle
            x={-155}
            y={116}
            w={310}
            label="Fuente"
            options={providers.map((p) => ({ value: p.id, label: p.name }))}
            value={draftProvider ?? providers[0].id}
            onchange={(id) => (draftProvider = id)}
          />
        {/if}
        {@const listTop = providers.length > 1 ? 152 : 128}
        <McText text="Categorías (el mod debe tener todas las marcadas)" x={-155} y={listTop - 12} w={310} color="#a0a0a0" maxLines={1} />
        <McList
          items={categoryChoices}
          getId={(c) => c}
          row={categoryRow}
          bind:selectedId={categoryCursor}
          onrowclick={toggleCategory}
          onactivate={toggleCategory}
          expandable={false}
          x={-(bw >> 1)}
          y={listTop}
          w={bw}
          h={Math.max(24, gui.H - listTop - 34)}
          rowHeight={14}
          empty="Este proveedor no ofrece categorías."
          ariaLabel="Categorías"
        />
        <McButton anchor="bottom" x={-155} y={-28} w={150} label="Aplicar" onclick={applyFilters} />
        <McButton anchor="bottom" x={5} y={-28} w={150} label="Quitar filtros" onclick={clearDraft} />
      {:else}
        {#if totalText}
          <McText text={totalText} x={-155} y={92} w={310} color="#808080" maxLines={1} />
        {/if}
        <McList
          items={mods.results}
          getId={hitKey}
          row={hitRow}
          bind:selectedId={hitId}
          x={-(bw >> 1)}
          y={104}
          w={bw}
          h={gui.H - 166}
          loading={mods.busy === 'search' && mods.results.length === 0}
          loadingText="Buscando..."
          empty={mods.searched ? 'No se encontraron mods con estos filtros.' : 'Escribe qué buscas y pulsa Buscar.'}
          ariaLabel="Resultados de la búsqueda"
        />
        {#if mods.busy !== null && mods.busy !== 'search'}
          <McProgress percent={null} label="Instalando" x={-155} w={310} y={gui.H - 42} />
        {:else if messages.length > 0}
          {#each messages as m, i (m.text)}
            <McText text={m.text} x={-155} y={gui.H - 52 + i * 10} w={310} align="center" color={m.color} role={m.role} maxLines={1} />
          {/each}
        {:else}
          {#each hitDetails as line, i (i)}
            <McText text={line} x={-155} y={gui.H - 52 + i * 10} w={310} align="center" color="#a0a0a0" maxLines={1} />
          {/each}
        {/if}
        <McButton
          anchor="bottom"
          x={-155}
          y={-28}
          w={102}
          label={selectedHit && mods.busy === selectedHit.projectId ? 'Instalando…' : 'Instalar'}
          ariaLabel={selectedHit ? `Instalar ${selectedHit.title}` : 'Instalar mod'}
          disabled={locked || !selectedHit || installedProjects.has(hitKey(selectedHit))}
          onclick={() => selectedHit && mods.install(instance.id, selectedHit)}
        />
        <McButton
          anchor="bottom"
          x={-51}
          y={-28}
          w={102}
          label="Ver más"
          ariaLabel="Cargar más resultados"
          disabled={mods.busy !== null || !mods.hasMore}
          onclick={() => mods.loadMore(instance.id)}
        />
        <McButton anchor="bottom" x={53} y={-28} w={102} label="Volver" onclick={() => goBack()} />
      {/if}
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
