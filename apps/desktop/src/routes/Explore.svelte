<script lang="ts">
  import McButton from '../lib/mc/McButton.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { findItemIcon } from '../lib/itemIcons'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  interface Choice {
    id: 'vanilla' | 'mods' | 'pack'
    icon: string
    title: string
    tagline: string
    description: string
    action: string
  }

  const choices: Choice[] = [
    {
      id: 'vanilla',
      icon: 'grass-block',
      title: 'Sin mods · La esencia de Minecraft',
      tagline: 'Minecraft original, sin cambios',
      description: 'Minecraft original, tal y como es. Elige la versión que quieras y empieza a jugar.',
      action: 'Crear instancia',
    },
    {
      id: 'mods',
      icon: 'diamond-pickaxe',
      title: 'Con mods · Un mundo a tu medida',
      tagline: 'Fabric, Quilt, Forge o NeoForge',
      description: 'Instancias con Fabric (también Quilt, Forge y NeoForge) para añadir mods: buscas en Modrinth y se instalan con lo que necesitan.',
      action: 'Crear instancia',
    },
    {
      id: 'pack',
      icon: 'crafting-table',
      title: 'Modpack · Importa un modpack',
      tagline: 'Archivos .mrpack de Modrinth',
      description: 'Si tienes un archivo .mrpack (Modrinth), se crea la instancia con todo lo necesario.',
      action: 'Importar modpack...',
    },
  ]

  let selectedId = $state<string>('vanilla')
  let importing = $state(false)
  let error = $state<string | null>(null)

  const selected = $derived(choices.find((c) => c.id === selectedId) ?? choices[0])

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

  function run(choice: Choice) {
    if (choice.id === 'vanilla') nav.create('vanilla')
    else if (choice.id === 'mods') nav.create('fabric')
    else importPack()
  }

  function row(c: Choice): RowSpec {
    const icon = findItemIcon(c.icon)
    return { icon: icon ? { src: icon.src } : undefined, lines: [{ text: c.title }, { text: c.tagline }] }
  }
</script>

<McScreen title="Explorar">
  <McList
    items={choices}
    getId={(c) => c.id}
    {row}
    bind:selectedId
    onactivate={run}
    x={-(bw >> 1)}
    y={24}
    w={bw}
    h={116}
    ariaLabel="Formas de empezar"
  />
  <McText text={selected.description} x={-155} y={142} w={310} color="#a0a0a0" maxLines={2} />

  {#if importing && instances.importPercent !== null}
    <McProgress percent={instances.importPercent} label="Importando modpack" x={-155} w={310} y={gui.H - 64} />
  {/if}
  {#if error}
    <McText text={error} x={-155} y={gui.H - 78} w={310} color="#ff5555" maxLines={2} role="alert" />
  {/if}

  <McButton anchor="bottom" y={-52} label={importing ? 'Importando…' : selected.action} disabled={importing} onclick={() => run(selected)} />
  <McButton anchor="bottom" y={-28} label="Atrás" onclick={() => nav.go('menu')} />
</McScreen>
