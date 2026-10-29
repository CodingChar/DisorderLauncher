<script lang="ts">
  import McDialog from './McDialog.svelte'
  import { fitText } from './font'
  import { detail } from './detail.svelte'
</script>

{#if detail.open}
  <McDialog title={fitText(detail.title, 280)} message={detail.text} scroll buttons={[{ label: 'Cerrar', onclick: () => detail.close() }]} onclose={() => detail.close()} />
{/if}
