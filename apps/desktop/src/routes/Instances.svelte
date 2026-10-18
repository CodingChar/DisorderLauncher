<script lang="ts">
  import McButton from '../lib/mc/McButton.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { defaultIconFor, findItemIcon } from '../lib/itemIcons'
  import { loaderName, versionLabel } from '../lib/text'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'
  import type { Instance } from '../lib/types'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const selected = $derived(instances.selected)
  const anyBusy = $derived(instances.list.some((i) => sessions.isBusy(i.id)))
  const canPlay = $derived(!!selected && !(anyBusy && !sessions.isBusy(selected.id)))

  function play(id: string) {
    instances.select(id)
    // Si ya esta abierta solo se muestra su sesion; si no, se inicia y se pasa al centro de sesion.
    if (sessions.get(id)?.phase !== 'running') sessions.launch(id)
    nav.go('home')
  }

  function configure(id: string) {
    instances.select(id)
    nav.go('edit')
  }

  function row(i: Instance): RowSpec {
    const running = sessions.get(i.id)?.phase === 'running'
    const profile = accounts.list.find((a) => a.id === i.profileId)
    const icon = findItemIcon(i.icon) ?? defaultIconFor(i.loader)
    return {
      icon: icon ? { src: icon.src } : undefined,
      lines: [
        { text: running ? `${i.name}  (En marcha)` : i.name, color: running ? '#55ff55' : '#ffffff' },
        { text: `${versionLabel(i.minecraftVersion)} · ${loaderName(i.loader)}` },
        { text: profile ? `Perfil: ${profile.label}` : '' },
      ],
    }
  }
</script>

<McScreen title="Mis instancias">
  <McList
    items={instances.list}
    getId={(i) => i.id}
    {row}
    selectedId={selected?.id ?? null}
    onselect={(i) => instances.select(i.id)}
    onactivate={(i) => !(anyBusy && !sessions.isBusy(i.id)) && play(i.id)}
    x={-(bw >> 1)}
    y={24}
    w={bw}
    h={gui.H - 94}
    empty="Aún no tienes instancias. Crea la primera con «Nueva instancia»."
    ariaLabel="Mis instancias"
  />
  {#if anyBusy}
    <McText text="Hay una instancia en marcha. Cuando termine podrás iniciar otra." x={-155} y={gui.H - 66} w={310} align="center" color="#808080" role="status" />
  {/if}

  <McButton anchor="bottom" x={-154} y={-52} w={150} label="Jugar seleccionada" disabled={!canPlay} onclick={() => selected && play(selected.id)} />
  <McButton anchor="bottom" x={4} y={-52} w={150} label="Nueva instancia" onclick={() => nav.create()} />
  <McButton anchor="bottom" x={-150} y={-28} w={72} label="Editar" disabled={!selected} onclick={() => selected && configure(selected.id)} />
  <McButton anchor="bottom" x={-74} y={-28} w={72} label="Mods" disabled={!selected} onclick={() => selected && (instances.select(selected.id), nav.go('mods'))} />
  <McButton anchor="bottom" x={2} y={-28} w={72} label="Explorar" onclick={() => nav.go('explore')} />
  <McButton anchor="bottom" x={78} y={-28} w={72} label="Atrás" onclick={() => nav.go('menu')} />
</McScreen>
