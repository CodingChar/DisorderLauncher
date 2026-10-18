<script lang="ts">
  import McButton from '../../lib/mc/McButton.svelte'
  import McLabel from '../../lib/mc/McLabel.svelte'
  import McList from '../../lib/mc/McList.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import McToggle from '../../lib/mc/McToggle.svelte'
  import { useGui } from '../../lib/mc/gui'
  import type { RowSpec } from '../../lib/mc/listTypes'
  import { bandWidth } from '../../lib/mc/pos'
  import type { CustomVersions } from '../../lib/customVersions.svelte'
  import { externalKindName } from '../../lib/text'
  import type { ExternalVersion } from '../../lib/types'

  interface Props {
    custom: CustomVersions
    disabled?: boolean
    onback: () => void
  }

  let { custom, disabled = false, onback }: Props = $props()

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))
  const versions = $derived(custom.scan?.versions ?? [])

  function row(v: ExternalVersion): RowSpec {
    return {
      lines: [
        { text: `${externalKindName(v.kind)} · ${v.id}` },
        { text: `${v.baseVersion ? `Minecraft ${v.baseVersion}` : 'Versión independiente'}${v.imported ? ' · ya usada antes' : ''}` },
      ],
    }
  }

  const emptyText = $derived(
    custom.scan && custom.scan.found
      ? 'No hay versiones de OptiFine, Forge, Fabric u otras personalizadas en esta carpeta.'
      : 'No encontramos una carpeta de Minecraft en este equipo. Si la tienes en otro lugar, búscala.',
  )
</script>

<McScreen title="Versión instalada">
  <McList
    items={versions}
    getId={(v) => v.id}
    {row}
    selectedId={custom.value || null}
    onselect={(v) => !disabled && (custom.value = v.id)}
    onactivate={(v) => {
      if (disabled) return
      custom.value = v.id
      onback()
    }}
    x={-(bw >> 1)}
    y={22}
    w={bw}
    h={gui.H - 122}
    rowHeight={24}
    loading={custom.loading}
    loadingText="Buscando versiones instaladas fuera del launcher..."
    empty={emptyText}
    ariaLabel="Versiones instaladas fuera del launcher"
  />

  {#if custom.error}
    <McText text={custom.error} x={-155} y={gui.H - 96} w={310} color="#ff5555" maxLines={1} role="alert" />
  {:else if custom.scan?.found}
    <McLabel x={-155} y={gui.H - 96} color="#808080" max={310} text={`Carpeta: ${custom.scan.dir}`} />
  {/if}
  <McToggle x={-155} w={150} y={gui.H - 84} label="Copiar datos" bind:checked={custom.copyData} {disabled} />
  <McButton x={5} w={150} y={gui.H - 84} label="Elegir carpeta..." ariaLabel="Buscar en otra carpeta" {disabled} onclick={() => custom.pickFolder()} />
  <McText text="Copia también tus mundos, mods y ajustes. Tu carpeta original no se toca." x={-155} y={gui.H - 60} w={310} color="#707070" maxLines={2} />
  <McButton anchor="bottom" y={-28} label="Hecho" onclick={onback} />
</McScreen>
