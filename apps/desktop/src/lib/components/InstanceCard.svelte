<script lang="ts">
  import { loaderName } from '../text'
  import type { Instance } from '../types'
  import ItemLogo from './ItemLogo.svelte'

  interface Props {
    instance: Instance
    selected: boolean
    running: boolean
    busy: boolean
    onplay: () => void
    onconfigure: () => void
    onselect: () => void
  }

  let { instance, selected, running, busy, onplay, onconfigure, onselect }: Props = $props()

  const memory = $derived(
    instance.memory.mode === 'fixed'
      ? `${Math.round((instance.memory.maxMb / 1024) * 10) / 10} GB de memoria`
      : 'Memoria automática',
  )
</script>

<section class="instance-card" class:selected aria-label={instance.name}>
  <div class="instance-top">
    <ItemLogo loader={instance.loader} icon={instance.icon} />
    <span class="loader-badge">{loaderName(instance.loader)}</span>
    {#if running}
      <span class="active-instance"><span class="status-dot"></span>En ejecución</span>
    {/if}
  </div>

  <h4 title={instance.name}>{instance.name}</h4>
  <p>Minecraft {instance.minecraftVersion}</p>

  <div class="library-specs">
    <span>{memory}</span>
    <span>{instance.java.mode === 'path' ? 'Java propio' : 'Java automático'}</span>
  </div>

  <div class="library-actions">
    <button class="secondary" onclick={onconfigure} disabled={busy}>Configurar</button>
    <button class="primary" onclick={onplay} disabled={busy && !running}>
      {running ? 'Ver sesión' : 'Jugar'}
    </button>
  </div>

  <div class="instance-bottom">
    <button class="text-link" onclick={onselect} aria-pressed={selected}>
      {selected ? 'Instancia activa' : 'Usar como activa'}
    </button>
  </div>
</section>
