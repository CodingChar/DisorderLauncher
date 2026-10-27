<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { playSfx } from '../audio/sfx'

  interface Props {
    /** Posicion CSS (`left`/`top`) dentro del contenedor positivo mas cercano. */
    style: string
    label: string
    onclick: () => void
  }

  let { style, label, onclick }: Props = $props()
</script>

<!-- Marca «[+]»: el texto de al lado esta acortado y aqui se abre completo. -->
<button
  type="button"
  class="mc-more"
  data-audit-small
  {style}
  aria-label={label}
  title={label}
  onclick={(event) => {
    event.stopPropagation()
    playSfx('click')
    onclick()
  }}
  ondblclick={(event) => event.stopPropagation()}
><BitmapText text="[+]" color="#ffff55" sr={false} /></button>
