<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import BitmapText from './BitmapText.svelte'
  import { useGui } from './gui'

  interface Props {
    title: string
    children: Snippet
  }

  let { title, children }: Props = $props()

  const gui = useGui()
  let heading = $state<HTMLElement>()

  // Al abrir una pantalla el foco pasa a su titulo: teclado y lectores de pantalla no se quedan en un boton que ya no existe.
  onMount(() => heading?.focus({ preventScroll: true }))
</script>

<div class="mc-screen">
  <h1 bind:this={heading} class="sr-only" tabindex="-1">{title}</h1>
  <div class="mc-label" style="left:calc(var(--cx) * 1px);top:8px" aria-hidden="true"><BitmapText text={title} center max={gui.W - 8} sr={false} /></div>
  {@render children()}
</div>
