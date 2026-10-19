<script lang="ts">
  import McButton from '../../lib/mc/McButton.svelte'
  import McCycle from '../../lib/mc/McCycle.svelte'
  import McList from '../../lib/mc/McList.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import { useGui } from '../../lib/mc/gui'
  import type { RowSpec } from '../../lib/mc/listTypes'
  import { bandWidth } from '../../lib/mc/pos'
  import { SHOW_LEVELS, formatReleaseDate, versionKindLabel } from '../../lib/versionPicker'
  import { versions } from '../../lib/stores/versions.svelte'
  import type { LoaderKind, VersionEntry } from '../../lib/types'

  interface Props {
    loader: LoaderKind
    /** Version elegida. */
    value: string
    entries: VersionEntry[]
    level: (typeof SHOW_LEVELS)[number]['value']
    disabled?: boolean
    onchange: (id: string) => void
    onlevel: (level: (typeof SHOW_LEVELS)[number]['value']) => void
    onback: () => void
  }

  let { loader, value, entries, level, disabled = false, onchange, onlevel, onback }: Props = $props()

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const failed = $derived(!!versions.error && versions.list.length === 0 && !versions.loading)
  const none = $derived(versions.list.length === 0 && !versions.loading && !versions.error)

  function row(v: VersionEntry): RowSpec {
    const date = formatReleaseDate(v.releaseTime)
    return { lines: [{ text: v.id }, { text: [versionKindLabel[v.type], date].filter(Boolean).join(' · ') }] }
  }

  const notice = $derived(
    versions.source === 'cached'
      ? 'Sin conexión: se muestra la última lista descargada.'
      : versions.source === 'installed'
        ? 'Sin conexión: solo aparecen las versiones que ya tienes instaladas.'
        : '',
  )
</script>

<McScreen title="Versión de Minecraft">
  {#if failed}
    <McText text={versions.error ?? ''} x={-155} y={60} w={310} align="center" color="#ff5555" maxLines={5} role="alert" />
    <McButton y={120} label="Reintentar" onclick={() => versions.load(loader, true)} />
  {:else if none}
    <McText text="No hay versiones disponibles para este tipo de instancia. Conéctate a internet para ver la lista completa." x={-155} y={60} w={310} align="center" color="#a0a0a0" maxLines={4} />
    <McButton y={120} label="Buscar de nuevo" onclick={() => versions.load(loader, true)} />
  {:else}
    <McCycle label="Mostrar" options={SHOW_LEVELS} value={level} onchange={onlevel} y={22} {disabled} />
    <McList
      items={entries}
      getId={(v) => v.id}
      {row}
      selectedId={value || null}
      onselect={(v) => !disabled && onchange(v.id)}
      onactivate={(v) => {
        if (disabled) return
        onchange(v.id)
        onback()
      }}
      x={-(bw >> 1)}
      y={46}
      w={bw}
      h={gui.H - 116}
      rowHeight={24}
      loading={versions.loading && versions.list.length === 0}
      loadingText="Buscando versiones de Minecraft..."
      empty="No hay versiones de este tipo."
      ariaLabel="Versiones de Minecraft"
    />
    {#if notice}<McText text={notice} x={-155} y={gui.H - 62} w={310} align="center" color="#ffaa00" role="status" maxLines={2} />{/if}
  {/if}
  <McButton anchor="bottom" y={-28} label="Hecho" onclick={onback} />
</McScreen>
