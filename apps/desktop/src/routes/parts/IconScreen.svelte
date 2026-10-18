<script lang="ts">
  import McButton from '../../lib/mc/McButton.svelte'
  import McIconGrid from '../../lib/mc/McIconGrid.svelte'
  import type { GridIcon } from '../../lib/mc/listTypes'
  import McLabel from '../../lib/mc/McLabel.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import { ITEM_ICONS, defaultIconFor } from '../../lib/itemIcons'
  import type { LoaderKind } from '../../lib/types'

  interface Props {
    /** Id del icono elegido; cadena vacia = el del tipo de instancia. */
    value: string
    loader: LoaderKind
    disabled?: boolean
    onback: () => void
  }

  let { value = $bindable(), loader, disabled = false, onback }: Props = $props()

  const icons = $derived<GridIcon[]>([{ id: '', label: 'Por defecto', src: defaultIconFor(loader)?.src ?? '' }, ...ITEM_ICONS])
  const current = $derived(icons.find((i) => i.id === value) ?? icons[0])
</script>

<McScreen title="Icono de la instancia">
  <McIconGrid {icons} bind:value x={0} y={26} {disabled} ariaLabel="Icono de la instancia" />
  <McLabel center y={112} text={current.label} />
  <McText text="El icono por defecto depende del tipo de instancia." x={-155} y={128} w={310} align="center" color="#808080" />
  <McButton anchor="bottom" y={-28} label="Hecho" onclick={onback} />
</McScreen>
