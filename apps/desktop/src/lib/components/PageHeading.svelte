<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    eyebrow: string
    title: string
    description?: string
    /** Acciones a la derecha del titulo (p. ej. «Nueva instancia»). */
    actions?: Snippet
  }

  let { eyebrow, title, description, actions }: Props = $props()
</script>

<div class="page-heading">
  <div>
    <div class="eyebrow">{eyebrow}</div>
    <h1>{title}</h1>
    {#if description}<p>{description}</p>{/if}
  </div>
  {#if actions}{@render actions()}{/if}
</div>
