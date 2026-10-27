<script lang="ts">
  import McButton from '../lib/mc/McButton.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { wrap } from '../lib/mc/font'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { api } from '../lib/api'
  import type { StorageSummary } from '../lib/types'
  import { FEATURES } from '../lib/features'
  import { findItemIcon } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const instance = $derived(instances.selected)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)

  let working = $state(false)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let selectedId = $state<string>('repair')
  let storage = $state<StorageSummary | null>(null)

  async function loadStorage() {
    storage = await api.storeSummary().catch(() => null)
  }
  $effect(() => {
    void loadStorage()
  })

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

  function repair() {
    if (!instance) return
    sessions.repair(instance.id)
    nav.go('home')
  }

  const exportPack = () =>
    run(async () => {
      if (!instance) return
      const summary = await api.modpacksExport(instance.id)
      return summary ? `Modpack guardado: ${summary.remoteFiles} mods por enlace y ${summary.bundledFiles} archivos incluidos.` : undefined
    })

  const importPack = () =>
    run(async () => {
      const created = await instances.importModpack()
      if (created) nav.go('home')
    })

  const exportInstance = (includeWorlds: boolean) =>
    run(async () => {
      if (!instance) return
      const summary = await instances.exportDpack(instance.id, includeWorlds)
      return summary ? `Instancia guardada: ${summary.files} archivos (${formatSize(summary.bytes)}).` : undefined
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
    soon?: boolean
  }

  const tools: Tool[] = $derived([
    { id: 'repair', icon: 'redstone', title: 'Reparar instancia', description: 'Comprueba los archivos de la instancia activa y vuelve a descargar lo que falte o esté dañado.' },
    { id: 'export', icon: 'ender-pearl', title: 'Exportar modpack', description: 'Guarda la instancia activa como un archivo .mrpack para compartirla. No incluye mundos ni registros.' },
    { id: 'import', icon: 'crafting-table', title: 'Importar modpack', description: 'Crea una instancia nueva a partir de un archivo .mrpack de Modrinth.' },
    { id: 'export7z', icon: 'diamond-pickaxe', title: 'Exportar instancia (.7z)', description: 'Guarda la instancia activa con todos sus mods en un solo archivo, para llevarla a otro equipo. No incluye mundos.' },
    { id: 'export7zWorlds', icon: 'grass-block', title: 'Exportar con mundos (.7z)', description: 'Igual que la anterior, pero también guarda tus mundos. Puede ocupar bastante más.' },
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
    { id: 'cache', icon: 'golden-apple', title: 'Limpiar caché', description: 'Borra descargas y archivos temporales que ya no usas.', soon: true },
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

  const canRun = $derived.by(() => {
    if (selected.soon || working) return false
    if (selected.id === 'repair') return !!instance && !busy
    if (selected.id === 'export' || selected.id === 'export7z' || selected.id === 'export7zWorlds') return !!instance && !busy
    return true
  })

  function execute() {
    if (!canRun) return
    if (selected.id === 'repair') repair()
    else if (selected.id === 'export') exportPack()
    else if (selected.id === 'import') importPack()
    else if (selected.id === 'export7z') exportInstance(false)
    else if (selected.id === 'export7zWorlds') exportInstance(true)
    else if (selected.id === 'import7z') importInstance()
    else if (selected.id === 'storage') cleanStorage()
  }
</script>

<McScreen title="Herramientas">
  <McLabel center y={22} color="#a0a0a0" max={gui.W - 8} text={instance ? `Instancia activa: ${instance.name}` : 'No hay ninguna instancia activa'} />
  <McList items={list} getId={(t) => t.id} {row} bind:selectedId onactivate={execute} x={-(bw >> 1)} y={34} w={bw} h={gui.H - 118} ariaLabel="Herramientas" />

  {#if working && instances.importPercent !== null}
    <McProgress percent={instances.importPercent} label={instances.importLabel} x={-155} w={310} y={gui.H - 64} />
  {:else if working && instances.exportPercent !== null}
    <McProgress percent={instances.exportPercent} label="Guardando la instancia" x={-155} w={310} y={gui.H - 64} />
  {/if}
  {#if error}
    <McText text={error} x={-155} y={gui.H - 78} w={310} color="#ff5555" maxLines={2} role="alert" />
  {:else if notice}
    <McText text={notice} x={-155} y={gui.H - 78} w={310} color="#55ff55" maxLines={2} role="status" />
  {/if}

  <McButton anchor="bottom" y={-52} label={working ? 'Trabajando…' : 'Ejecutar'} ariaLabel={`Ejecutar: ${selected.title}`} disabled={!canRun} onclick={execute} />
  <McButton anchor="bottom" y={-28} label="Atrás" onclick={() => nav.go('menu')} />
</McScreen>
