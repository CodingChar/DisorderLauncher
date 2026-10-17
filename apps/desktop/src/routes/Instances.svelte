<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Icon from '../lib/components/Icon.svelte'
  import InstanceCard from '../lib/components/InstanceCard.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'

  const count = $derived(instances.list.length)
  const anyBusy = $derived(instances.list.some((i) => sessions.isBusy(i.id)))

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
</script>

<PageHeading eyebrow="DISORDERLAUNCHER" title="Mis instancias" description="Crea, configura y juega tus mundos de Minecraft.">
  {#snippet actions()}
    <Button onclick={() => nav.create()}><Icon name="plus" size={17} /> Nueva instancia</Button>
  {/snippet}
</PageHeading>

{#if count === 0}
  <p class="library-notice">
    Aún no tienes instancias. Crea la primera con «Nueva instancia» o importa un modpack desde <button class="text-link" onclick={() => nav.go('explore')}>Explorar</button>.
  </p>
{:else}
  <div class="library-toolbar">
    <span>{count} {count === 1 ? 'instancia' : 'instancias'} en tu biblioteca</span>
    <span class="tiny-label">CONFIGURACIONES INDEPENDIENTES</span>
  </div>

  <div class="instance-grid">
    {#each instances.list as item (item.id)}
      <InstanceCard
        instance={item}
        selected={instances.selected?.id === item.id}
        running={sessions.get(item.id)?.phase === 'running'}
        busy={anyBusy && !sessions.isBusy(item.id)}
        onplay={() => play(item.id)}
        onconfigure={() => configure(item.id)}
        onselect={() => instances.select(item.id)}
      />
    {/each}
  </div>

  {#if anyBusy}
    <p class="library-notice">Hay una instancia en marcha. Cuando termine podrás iniciar otra.</p>
  {/if}
{/if}
