<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import McViewport from '../lib/mc/McViewport.svelte'
  import SkinPreview3D from '../lib/mc/SkinPreview3D.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { skins } from '../lib/stores/skins.svelte'
  import type { SkinAsset, SkinModel } from '../lib/types'

  const gui = useGui()

  const localProfiles = $derived(accounts.list.filter((a) => a.provider === 'local'))

  let sub = $state<'main' | 'more' | 'rename'>('main')
  let profileId = $state('')
  let selectedId = $state<string | null>(null)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let busy = $state(false)
  let toRemove = $state<SkinAsset | null>(null)
  let newName = $state('')
  let yaw = $state(-28)

  const models = [
    { value: 'classic' as SkinModel, label: 'Clásico' },
    { value: 'slim' as SkinModel, label: 'Fino' },
  ]

  onMount(() => {
    skins.load()
  })

  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  // El perfil elegido arranca en el predeterminado si es local, o en el primero local.
  $effect(() => {
    if (profileId && localProfiles.some((a) => a.id === profileId)) return
    const preferred = localProfiles.find((a) => a.id === accounts.defaultId) ?? localProfiles[0]
    profileId = preferred?.id ?? ''
  })

  const profile = $derived(localProfiles.find((a) => a.id === profileId))
  const profileSkinId = $derived(profile?.providerData.type === 'local' ? profile.providerData.skinId : null)

  // La seleccion arranca en la skin del perfil (o la primera) y se corrige si la skin desaparece.
  $effect(() => {
    if (selectedId && skins.list.some((s) => s.id === selectedId)) return
    selectedId = (profileSkinId && skins.list.find((s) => s.id === profileSkinId)?.id) || skins.list[0]?.id || null
  })

  const selected = $derived(skins.list.find((s) => s.id === selectedId) ?? null)
  const selectedSrc = $derived(selected ? (skins.previews[selected.id] ?? '') : '')
  $effect(() => {
    const id = selected?.id
    if (id) untrack(() => { void skins.ensurePreview(id) })
  })
  const inUse = $derived(!!selected && profileSkinId === selected.id)

  async function run(action: () => Promise<string | void>) {
    if (busy) return
    busy = true
    error = null
    notice = null
    try {
      const message = await action()
      if (message) notice = message
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }

  const openEditor = (id: string | null) => {
    skins.editingId = id
    nav.go('skinEditor')
  }

  const importSkin = () =>
    run(async () => {
      const skin = await skins.importSkin()
      if (skin) selectedId = skin.id
      return skin ? `Se agregó «${skin.name}».` : undefined
    })

  const use = (skin: SkinAsset) =>
    run(async () => {
      await skins.assign(profileId, skin.id)
      return `«${skin.name}» es ahora la skin de ${profile?.label}.`
    })

  const unassign = () =>
    run(async () => {
      await skins.assign(profileId, null)
      return `${profile?.label} ya no tiene skin asignada.`
    })

  const exportSkin = (skin: SkinAsset) => run(async () => ((await skins.exportSkin(skin.id)) ? 'Skin guardada.' : undefined))

  const confirmRemove = () => {
    const target = toRemove
    toRemove = null
    if (target) run(async () => (await skins.remove(target.id), 'Skin eliminada.'))
  }

  const saveRename = () => {
    const target = selected
    if (!target) return
    sub = 'more'
    run(() => skins.rename(target.id, newName))
  }

  function row(skin: SkinAsset): RowSpec {
    const src = skins.previews[skin.id]
    return {
      icon: src
        ? [
            { src, crop: [8, 8, 8, 8], imageWidth: skin.width },
            { src, crop: [40, 8, 8, 8], imageWidth: skin.width },
          ]
        : undefined,
      lines: [
        { text: skin.name },
        { text: skin.model === 'slim' ? 'Brazos finos (Alex)' : 'Brazos clásicos (Steve)' },
        { text: profileSkinId === skin.id ? 'En uso' : '', color: '#55ff55' },
      ],
    }
  }

  const listH = $derived(gui.H - 124)
  const previewH = $derived(listH - 24)
  const unit = $derived(Math.max(1, Math.min(4, Math.floor(previewH / 38))))
</script>

{#if sub === 'rename' && selected}
  <McScreen title="Renombrar skin">
    <form
      class="mc-form"
      onsubmit={(e) => {
        e.preventDefault()
        if (newName.trim().length > 0) saveRename()
      }}
    >
      <McLabel x={-100} y={40} color="#a0a0a0" text="Nuevo nombre" />
      <McField x={-100} y={50} w={200} maxlength={48} bind:value={newName} autofocus label="Nuevo nombre de la skin" />
      <McButton anchor="bottom" x={-100} y={-28} w={98} type="submit" label="Guardar nombre" disabled={newName.trim().length === 0} />
      <McButton anchor="bottom" x={2} y={-28} w={98} label="Cancelar" onclick={() => (sub = 'more')} />
    </form>
  </McScreen>
{:else if sub === 'more' && selected}
  <McScreen title="Más opciones de skin">
    <McLabel center y={22} color="#a0a0a0" max={gui.W - 8} text={selected.name} />
    <McButton y={36} label="Renombrar..." disabled={busy} onclick={() => ((newName = selected.name), (sub = 'rename'))} />
    <McCycle y={60} label="Modelo" options={models} value={selected.model} disabled={busy} onchange={(m) => run(() => skins.setModel(selected.id, m))} />
    <McButton y={84} label="Exportar..." disabled={busy} onclick={() => exportSkin(selected)} />
    <McButton y={108} label="Eliminar..." disabled={busy} onclick={() => (toRemove = selected)} />
    {#if error}
      <McText text={error} x={-155} y={134} w={310} align="center" color="#ff5555" maxLines={2} role="alert" />
    {:else if skins.error}
      <McText text={skins.error} x={-155} y={gui.H - 76} w={310} color="#ff5555" maxLines={2} role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={134} w={310} align="center" color="#55ff55" maxLines={2} role="status" />
    {/if}
    <McText
      text="La skin se guarda aquí y se aplica dentro del juego al abrir una instancia de este perfil (Minecraft decide si los brazos son anchos o finos). En versiones anteriores a 1.8 la skin se adapta sola: sin segunda capa y con los brazos y piernas iguales. No se registra en tu cuenta de Microsoft."
      x={-155}
      y={gui.H - 94}
      w={310}
      color="#707070"
      maxLines={6}
    />
    <McButton anchor="bottom" y={-28} label="Hecho" onclick={() => (sub = 'main')} />

    {#if toRemove}
      <McDialog
        title="¿Eliminar esta skin?"
        message={`Se borrará «${toRemove.name}» de tu biblioteca y se quitará de los perfiles que la usan.`}
        buttons={[
          { label: 'Cancelar', onclick: () => (toRemove = null) },
          {
            label: 'Eliminar',
            onclick: () => {
              confirmRemove()
              sub = 'main'
            },
          },
        ]}
        cancelIndex={0}
        onclose={() => (toRemove = null)}
      />
    {/if}
  </McScreen>
{:else}
  <McScreen title="Skins">
    {#if localProfiles.length > 0}
      <McCycle x={-155} y={20} w={190} label="Perfil" options={localProfiles.map((a) => ({ value: a.id, label: a.label }))} bind:value={profileId} disabled={busy || localProfiles.length < 2} />
    {:else}
      <McText text="Crea un perfil sin conexión para asignarle una skin." x={-155} y={22} w={310} color="#ffaa00" maxLines={2} />
    {/if}

    <McList
      items={skins.list}
      getId={(s) => s.id}
      {row}
      onvisible={(visible) => { void skins.ensurePreviews(visible.map((skin) => skin.id)) }}
      bind:selectedId
      onactivate={(s) => !busy && profile && use(s)}
      x={-155}
      y={44}
      w={190}
      h={listH}
      loading={!skins.loaded}
      empty="Aún no tienes skins. Importa un PNG de 64×64 (o 64×32) o crea una desde cero."
      ariaLabel="Tus skins"
    />

    <McViewport x={41} y={44} w={114} h={previewH}>
      {#if selected && selectedSrc}
        <SkinPreview3D src={selectedSrc} model={selected.model} height={selected.height} {unit} bind:yaw />
      {/if}
    </McViewport>
    <McButton x={76} y={44 + previewH + 3} w={20} label="<" ariaLabel="Girar a la izquierda" disabled={!selectedSrc} onclick={() => (yaw -= 45)} />
    <McButton x={100} y={44 + previewH + 3} w={20} label=">" ariaLabel="Girar a la derecha" disabled={!selectedSrc} onclick={() => (yaw += 45)} />

    {#if error}
      <McText text={error} x={-155} y={gui.H - 76} w={310} color="#ff5555" maxLines={2} role="alert" />
    {:else if skins.error}
      <McText text={skins.error} x={-155} y={gui.H - 76} w={310} color="#ff5555" maxLines={2} role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={gui.H - 76} w={310} color="#55ff55" maxLines={2} role="status" />
    {/if}

    <McButton anchor="bottom" x={-150} y={-52} w={72} label="Usar" ariaLabel="Usar esta skin en el perfil" disabled={busy || !profile || !selected || inUse} onclick={() => selected && use(selected)} />
    <McButton anchor="bottom" x={-74} y={-52} w={72} label="Editar" disabled={busy || !selected} onclick={() => selected && openEditor(selected.id)} />
    <McButton anchor="bottom" x={2} y={-52} w={72} label="Importar..." ariaLabel="Importar skin" disabled={busy} onclick={importSkin} />
    <McButton anchor="bottom" x={78} y={-52} w={72} label="Crear..." ariaLabel="Crear skin nueva" disabled={busy} onclick={() => openEditor(null)} />
    <McButton anchor="bottom" x={-150} y={-28} w={72} label="Más..." ariaLabel="Más opciones de la skin" disabled={busy || !selected} onclick={() => (sub = 'more')} />
    <McButton anchor="bottom" x={-74} y={-28} w={72} label="Sin skin" ariaLabel="Quitar skin del perfil" disabled={busy || !profileSkinId} onclick={unassign} />
    <McButton anchor="bottom" x={2} y={-28} w={148} label="Atrás" ariaLabel="Volver al menú" onclick={() => nav.go('menu')} />
  </McScreen>
{/if}
