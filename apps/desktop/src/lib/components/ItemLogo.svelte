<script lang="ts">
  import { defaultIconFor, findItemIcon } from '../itemIcons'
  import type { LoaderKind } from '../types'

  /** `icon`: id elegido por la persona; sin el, se usa el icono del tipo de instancia. */
  let { loader, icon = null, size = 38 }: { loader: LoaderKind; icon?: string | null; size?: number } = $props()

  const shown = $derived(findItemIcon(icon) ?? defaultIconFor(loader))
</script>

{#if shown}
  <img class="item-logo" src={shown.src} width={size} height={size} alt="" aria-hidden="true" draggable="false" />
{:else}
  <span class="item-logo" style:width="{size}px" style:height="{size}px" aria-hidden="true"></span>
{/if}
