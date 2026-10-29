<script lang="ts">
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import McToggle from '../lib/mc/McToggle.svelte'
  import { wrap } from '../lib/mc/font'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { api } from '../lib/api'
  import type { StorageSummary } from '../lib/types'
  import type { Instance } from '../lib/types'
  import { FEATURES } from '../lib/features'
  import { defaultIconFor, findItemIcon } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import { loaderName, versionLabel } from '../lib/text'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  let working = $state(false)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let selectedId = $state<string>('repair')
  let storage = $state<StorageSummary | null>(null)

  /** `list`: elegir la herramienta. `pick`: elegir sobre que instancia se aplica. */
  let sub = $state<'list' | 'pick'>('list')
  let pickedId = $state<string | null>(null)
  let includeWorlds = $state(false)

  async function loadStorage() {
    storage = await api.storeSummary().catch(() => null)
  }
  $effect(() => {
    void loadStorage()
  })

  // Esc / Atras desde el selector de instancia vuelve a la lista de herramientas.
  $effect(() => {
    if (sub === 'list') return
    return pushBack(() => backToList())
  })

  function backToList() {
    sub = 'list'
    error = null
    notice = null
  }

  async function run(action: () => Promise<string | void>) {
    if (working) return
    working = true
    error = null
    notice = null
    try {
      const message = await action()
      if (message) notice = message
    } catch (e) {
      error = (e as Error).message
    } finally {
      working = false
    }
  }

  /** Tamano legible para personas: MB o GB, sin decimales de mas. */
  function formatSize(bytes: number): string {
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`
    return `${Math.max(1, Math.round(bytes / 1024 ** 2))} MB`
  }

  interface Tool {
    id: string
    icon: string
    title: string
    description: string
    /** `true` si trabaja sobre una instancia: se pide cual antes de empezar. */
    needsInstance?: boolean
    /** Pregunta del selector de instancia y texto del boton que ejecuta. */
    question?: string
    action?: string
    soon?: boolean
  }

  const tools: Tool[] = $derived([
    { id: 'repair', icon: 'redstone', title: 'Reparar instancia', description: 'Elige una instancia y vuelve a descargar los archivos del juego que falten o estén dañados.', needsInstance: true, question: '¿Qué instancia quieres reparar?', action: 'Reparar' },
    { id: 'export', icon: 'ender-pearl', title: 'Exportar modpack (.mrpack)', description: 'Elige una instancia y guárdala como .mrpack de Modrinth. No incluye mundos ni registros.', needsInstance: true, question: '¿Qué instancia quieres exportar como modpack?', action: 'Exportar' },
    { id: 'import', icon: 'crafting-table', title: 'Importar modpack (.mrpack)', description: 'Crea una instancia nueva a partir de un archivo .mrpack de Modrinth.' },
    { id: 'export7z', icon: 'diamond-pickaxe', title: 'Exportar instancia (.7z)', description: 'Elige una instancia y guárdala con todos sus mods en un solo archivo para llevarla a otro equipo. Puedes incluir tus mundos.', needsInstance: true, question: '¿Qué instancia quieres exportar?', action: 'Exportar' },
    { id: 'import7z', icon: 'diamond-sword', title: 'Importar instancia (.7z)', description: 'Crea una instancia nueva a partir de un archivo .7z exportado desde DisorderLauncher.' },
    {
      id: 'storage',
      icon: 'tnt',
      title: 'Liberar espacio',
      description: storage
        ? `Ahorras ${formatSize(storage.sharedBytes)} al compartir mods entre instancias. Se pueden liberar ${formatSize(storage.reclaimableBytes)} sin uso.`
        : 'Libera los archivos de mods que ya no usa ninguna instancia.',
    },
  ])

  /** Herramientas que todavia no existen: no se muestran (ver `FEATURES`) y, si se activan, nunca fingen funcionar. */
  const soon: Tool[] = [
    { id: 'backup', icon: 'tnt', title: 'Copia de seguridad', description: 'Guarda tus instancias y su configuración en un archivo comprimido.', soon: true },
    { id: 'diagnostic', icon: 'torch', title: 'Informe de diagnóstico', description: 'Resume tu equipo y las últimas sesiones para pedir ayuda.', soon: true },
  ]

  const list = $derived(FEATURES.comingSoonTools ? [...tools, ...soon] : tools)
  const selected = $derived(list.find((t) => t.id === selectedId) ?? list[0])
  const textWidth = $derived(bw - 10 - 37 - 2)

  function row(t: Tool): RowSpec {
    const icon = findItemIcon(t.icon)
    const description = wrap(t.description, textWidth - 1, t.soon ? 1 : 2)
    return {
      icon: icon ? { src: icon.src } : undefined,
      disabled: t.soon,
      lines: [{ text: t.title }, { text: description[0] ?? '' }, { text: t.soon ? 'PRÓXIMAMENTE' : (description[1] ?? '') }],
    }
  }

  // ---- Selector de instancia -------------------------------------------------

  const picked = $derived(instances.list.find((i) => i.id === pickedId) ?? null)
  const pickedBusy = $derived(picked ? sessions.isBusy(picked.id) : false)
  const askWorlds = $derived(sub === 'pick' && selected.id === 'export7z')

  function instanceRow(i: Instance): RowSpec {
    const icon = findItemIcon(i.icon) ?? defaultIconFor(i.loader)
    const busy = sessions.isBusy(i.id)
    return {
      icon: icon ? { src: icon.src } : undefined,
      lines: [
        { text: i.name },
        { text: `${versionLabel(i.minecraftVersion)} · ${loaderName(i.loader)}` },
        busy ? { text: 'En uso: ciérrala primero', color: '#ffaa00' } : { text: '' },
      ],
    }
  }

  function chooseInstance() {
    error = null
    notice = null
    // Con una sola instancia no hay nada que elegir; con varias, la persona decide.
    pickedId = instances.list.length === 1 ? instances.list[0].id : null
    includeWorlds = false
    sub = 'pick'
  }

  function runOnInstance() {
    if (!picked || pickedBusy || working) return
    const instance = picked
    if (selected.id === 'repair') {
      instances.select(instance.id)
      sessions.repair(instance.id)
      nav.go('home')
    } else if (selected.id === 'export') {
      void run(async () => {
        const summary = await api.modpacksExport(instance.id)
        return summary ? `«${instance.name}» guardada: ${summary.remoteFiles} mods por enlace y ${summary.bundledFiles} archivos incluidos.` : undefined
      })
    } else if (selected.id === 'export7z') {
      void run(async () => {
        const summary = await instances.exportDpack(instance.id, includeWorlds)
        return summary ? `«${instance.name}» guardada: ${summary.files} archivos (${formatSize(summary.bytes)}).` : undefined
      })
    }
  }

  // ---- Herramientas que no dependen de una instancia ---------------------------

  const importPack = () =>
    run(async () => {
      const created = await instances.importModpack()
      if (created) nav.go('home')
    })

  const importInstance = () =>
    run(async () => {
      const created = await instances.importDpack()
      if (created) nav.go('home')
    })

  const cleanStorage = () =>
    run(async () => {
      const { freedBytes } = await api.storeCleanup()
      await loadStorage()
      return freedBytes > 0 ? `Listo: se liberaron ${formatSize(freedBytes)}.` : 'No hay archivos sin uso que liberar ahora.'
    })

  const canRun = $derived.by(() => {
    if (selected.soon || working) return false
    if (selected.needsInstance) return instances.list.length > 0
    return true
  })

  function execute() {
    if (!canRun) return
    if (selected.needsInstance) chooseInstance()
    else if (selected.id === 'import') importPack()
    else if (selected.id === 'import7z') importInstance()
    else if (selected.id === 'storage') cleanStorage()
  }
</script>

{#if sub === 'pick'}
  <McScreen title={selected.title}>
    <McLabel center y={22} color="#ffffff" max={gui.W - 8} text={selected.question ?? '¿Qué instancia?'} />
    <McList
      items={instances.list}
      getId={(i) => i.id}
      row={instanceRow}
      bind:selectedId={pickedId}
      onactivate={() => runOnInstance()}
      x={-(bw >> 1)}
      y={34}
      w={bw}
      h={gui.H - (askWorlds ? 142 : 118)}
      empty="Aún no tienes instancias. Crea una en «Mis instancias»."
      ariaLabel="Instancias"
    />
    {#if askWorlds}
      <McToggle x={-155} y={gui.H - 104} w={310} label="Incluir mis mundos" bind:checked={includeWorlds} disabled={working} />
    {/if}

    {#if working && instances.exportPercent !== null}
      <McProgress percent={instances.exportPercent} label="Guardando la instancia" x={-155} w={310} y={gui.H - 64} />
    {/if}
    {#if error}
      <McText text={error} x={-155} y={gui.H - 78} w={310} color="#ff5555" maxLines={2} role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={gui.H - 78} w={310} color="#55ff55" maxLines={2} role="status" />
    {:else if !picked && instances.list.length > 1}
      <McText text="Elige una instancia de la lista." x={-155} y={gui.H - 78} w={310} color="#a0a0a0" />
    {/if}

    <McButton
      anchor="bottom"
      y={-52}
      label={working ? 'Trabajando…' : picked ? `${selected.action}: ${picked.name}` : `${selected.action}…`}
      ariaLabel={picked ? `${selected.action} la instancia ${picked.name}` : 'Elige primero una instancia'}
      disabled={!picked || pickedBusy || working}
      onclick={runOnInstance}
    />
    <McButton anchor="bottom" y={-28} label="Atrás" onclick={backToList} />
  </McScreen>
{:else}
  <McScreen title="Herramientas">
    <McLabel center y={22} color="#a0a0a0" max={gui.W - 8} text="Elige una herramienta y pulsa el botón de abajo." />
    <McList items={list} getId={(t) => t.id} {row} bind:selectedId onactivate={execute} x={-(bw >> 1)} y={34} w={bw} h={gui.H - 118} ariaLabel="Herramientas" />

    {#if working && instances.importPercent !== null}
      <McProgress percent={instances.importPercent} label={instances.importLabel} x={-155} w={310} y={gui.H - 64} />
    {/if}
    {#if error}
      <McText text={error} x={-155} y={gui.H - 78} w={310} color="#ff5555" maxLines={2} role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={gui.H - 78} w={310} color="#55ff55" maxLines={2} role="status" />
    {/if}

    <McButton
      anchor="bottom"
      y={-52}
      label={working ? 'Trabajando…' : selected.needsInstance ? 'Elegir instancia…' : 'Ejecutar'}
      ariaLabel={selected.needsInstance ? `Elegir la instancia para: ${selected.title}` : `Ejecutar: ${selected.title}`}
      disabled={!canRun}
      onclick={execute}
    />
    <McButton anchor="bottom" y={-28} label="Atrás" onclick={() => nav.go('menu')} />
  </McScreen>
{/if}
