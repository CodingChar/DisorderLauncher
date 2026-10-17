<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import InstanceCard from '../lib/components/InstanceCard.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import { loaderLabel, phaseLabel } from '../lib/text'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'

  const instance = $derived(instances.selected)
  const session = $derived(instance ? sessions.get(instance.id) : undefined)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)
  const running = $derived(session?.phase === 'running')

  const percent = $derived.by(() => {
    const p = session?.progress
    return p && p.total > 0 ? (p.completed / p.total) * 100 : null
  })

  const statusText = $derived.by(() => {
    if (!session) return 'Listo para jugar'
    switch (session.phase) {
      case 'preparing':
        return 'Preparando…'
      case 'installing':
        return session.progress ? phaseLabel[session.progress.phase] : 'Descargando archivos…'
      case 'preparingJava':
        return 'Preparando componentes…'
      case 'starting':
        return 'Iniciando Minecraft…'
      case 'running':
        return 'Minecraft está abierto'
      case 'repairing':
        return session.progress ? phaseLabel[session.progress.phase] : 'Comprobando archivos…'
      case 'exited':
        if (session.message) return session.message
        return session.exitCode && session.exitCode !== 0
          ? 'Minecraft se cerró con un problema. Prueba a reparar la instancia.'
          : 'Minecraft se cerró'
      case 'failed':
        return session.message ?? 'No se pudo iniciar Minecraft.'
    }
  })

  const isProblem = $derived(
    session?.phase === 'failed' || (session?.phase === 'exited' && !!session.exitCode && session.exitCode !== 0),
  )
</script>

<div class="home">
  <header>
    <small>INICIO</small>
    <h1>DisorderLauncher</h1>
  </header>

  {#if instance}
    <Card variant="hero">
      <div class="hero">
        <span class="kicker">INSTANCIA ACTIVA</span>
        <h2>{instance.name}</h2>
        <p class="meta">{loaderLabel(instance.loader)} {instance.minecraftVersion}</p>

        <div class="actions">
          {#if running}
            <Button large onclick={() => sessions.stop(instance.id)}>Cerrar juego</Button>
          {:else}
            <Button variant="primary" large disabled={busy} onclick={() => sessions.launch(instance.id)}>
              {busy ? 'Un momento…' : 'JUGAR'}
            </Button>
          {/if}
          <Button variant="ghost" disabled={busy} onclick={() => sessions.repair(instance.id)}>Reparar</Button>
        </div>

        <div class="status" aria-live="polite">
          <p class:problem={isProblem}>{statusText}</p>
          {#if session && (session.phase === 'installing' || session.phase === 'repairing')}
            <ProgressBar {percent} label={statusText} />
          {/if}
        </div>
      </div>
    </Card>

    <section aria-labelledby="recent">
      <div class="section-head">
        <h3 id="recent">Tus instancias</h3>
        <Button variant="ghost" onclick={() => nav.go('create')}>+ Nueva</Button>
      </div>
      <div class="grid">
        {#each instances.list as item (item.id)}
          <InstanceCard instance={item} selected={item.id === instance.id} onselect={() => instances.select(item.id)} />
        {/each}
      </div>
    </section>
  {:else}
    <Card variant="hero">
      <div class="hero empty">
        <span class="kicker">TODO LISTO</span>
        <h2>Crea tu primera instancia</h2>
        <p class="meta">Una instancia es tu mundo de Minecraft con su propia versión. Elige la que quieras y juega.</p>
        <div class="actions">
          <Button variant="primary" large onclick={() => nav.go('create')}>Crear instancia</Button>
        </div>
      </div>
    </Card>
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  header small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.18em;
    margin-bottom: 5px;
  }

  header h1 {
    font-size: 34px;
    line-height: 0.95;
  }

  .hero {
    min-height: 240px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 6px;
  }

  .kicker {
    align-self: flex-start;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--text-secondary);
  }

  h2 {
    font-size: 40px;
    line-height: 0.98;
    letter-spacing: -0.06em;
    margin: 14px 0 0;
    overflow-wrap: anywhere;
  }

  .meta {
    margin: 0;
    color: var(--text-secondary);
    max-width: 560px;
    line-height: 1.6;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-top: 18px;
  }

  .status {
    min-height: 44px;
    margin-top: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .status p {
    margin: 0;
    font-size: 14px;
    color: var(--text-secondary);
  }

  .status p.problem {
    color: var(--text-primary);
    font-weight: 600;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  h3 {
    font-size: 18px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 12px;
  }
</style>
