<script lang="ts">
  import Avatar from '../lib/components/Avatar.svelte'
  import Button from '../lib/components/Button.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import ItemLogo from '../lib/components/ItemLogo.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import { loaderName, phaseLabel } from '../lib/text'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'

  const instance = $derived(instances.selected)
  const session = $derived(instance ? sessions.get(instance.id) : undefined)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)
  const running = $derived(session?.phase === 'running')
  const profile = $derived(
    (instance && accounts.list.find((a) => a.id === instance.profileId)) || accounts.current,
  )

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

  const pillText = $derived.by(() => {
    if (!session) return 'LISTA'
    if (session.phase === 'running') return 'EN EJECUCIÓN'
    if (session.phase === 'failed') return 'CON PROBLEMAS'
    if (session.phase === 'exited') return 'CERRADA'
    return 'PREPARANDO'
  })

  // Un fallo (o un cierre inesperado) se muestra en un dialogo con la accion adecuada.
  const crashed = $derived(session?.phase === 'exited' && !!session.exitCode && session.exitCode !== 0)
  const failure = $derived.by(() => {
    if (session?.phase === 'failed') {
      const message = session.message ?? 'No se pudo iniciar Minecraft.'
      return { message, retry: session.kind === 'offline', repair: session.kind === 'repairable', signIn: session.kind === 'signIn' }
    }
    if (crashed) {
      return { message: 'Minecraft se cerró de forma inesperada. Reparar la instancia suele solucionarlo.', retry: false, repair: true, signIn: false }
    }
    return null
  })

  function onFailureConfirm() {
    if (!instance || !failure) return
    const id = instance.id
    sessions.dismiss(id)
    if (failure.retry) sessions.launch(id)
    else if (failure.repair) sessions.repair(id)
    else if (failure.signIn) nav.go('accounts')
  }

  const isProblem = $derived(
    session?.phase === 'failed' || (session?.phase === 'exited' && !!session.exitCode && session.exitCode !== 0),
  )

  const memoryText = $derived(
    !instance
      ? ''
      : instance.memory.mode === 'fixed'
        ? `${Math.round((instance.memory.maxMb / 1024) * 10) / 10} GB como máximo`
        : 'Automática',
  )

  // ------------------------------------------------------------------ consola (salida real del juego)
  let filter = $state('')
  let level = $state<'all' | 'WARN' | 'ERROR'>('all')

  const lines = $derived(instance ? sessions.logOf(instance.id) : [])
  const visible = $derived(
    lines.filter((line) => {
      if (filter && !line.text.toLowerCase().includes(filter.toLowerCase())) return false
      if (level === 'all') return true
      return levelOf(line.text) === level
    }),
  )

  function levelOf(text: string): 'WARN' | 'ERROR' | 'INFO' {
    if (/\b(ERROR|FATAL|SEVERE)\b/.test(text)) return 'ERROR'
    if (/\b(WARN|WARNING)\b/.test(text)) return 'WARN'
    return 'INFO'
  }

  const emptyHeading = $derived(
    filter || level !== 'all' ? 'Sin resultados para este filtro.' : running || busy ? 'Esperando la salida del juego…' : 'Aún no hay salida del juego.',
  )
</script>

<PageHeading eyebrow="ESTADO DE LA SESIÓN" title="Centro de sesión" description="Tu instancia activa, su estado y lo que escribe el juego mientras está abierto." />

{#if instance}
  <section class="session-card panel" aria-label="Instancia activa">
    <div class="session-profile">
      <Avatar />
      <div>
        <strong>{profile?.label ?? 'Sin perfil'}</strong>
        <span>PERFIL DE LA SESIÓN</span>
      </div>
      <span class="pill">
        <span class="status-dot" class:warn={isProblem}></span>
        {pillText}
      </span>
    </div>

    <div class="session-identity">
      <ItemLogo loader={instance.loader} />
      <div>
        <h2>{instance.name}</h2>
        <p>Minecraft {instance.minecraftVersion} <span>·</span> {loaderName(instance.loader)}</p>
      </div>
      {#if session?.startedAt}<span class="session-started">Inicio: {session.startedAt}</span>{/if}
    </div>

    <div class="session-details">
      <span>Memoria <strong>{memoryText}</strong></span>
      <span>Java <strong>{instance.java.mode === 'path' ? 'Propio' : 'Automático'}</strong></span>
      <span>Tipo <strong>{instance.loader === 'vanilla' ? 'Original (sin mods)' : loaderName(instance.loader)}</strong></span>
      <span>Estado <strong>{statusText}</strong></span>
    </div>

    <div class="session-actions">
      {#if running}
        <Button large onclick={() => sessions.stop(instance.id)}>Cerrar juego</Button>
      {:else}
        <Button variant="primary" large disabled={busy} onclick={() => sessions.launch(instance.id)}>
          {busy ? 'Un momento…' : 'JUGAR'}
        </Button>
      {/if}
      {#if instance.loader !== 'vanilla'}
        <Button variant="ghost" disabled={busy} onclick={() => nav.go('mods')}>Mods</Button>
      {/if}
      <Button variant="ghost" disabled={busy} onclick={() => nav.go('edit')}>Configurar</Button>
      <Button variant="ghost" disabled={busy} onclick={() => sessions.repair(instance.id)}>Reparar</Button>
      <Button variant="ghost" onclick={() => nav.go('instances')}>Cambiar instancia</Button>
    </div>

    <div class="session-status" aria-live="polite">
      <p class:problem={isProblem}>{statusText}</p>
      {#if session && (session.phase === 'installing' || session.phase === 'repairing')}
        <ProgressBar {percent} label={statusText} />
      {/if}
    </div>
  </section>

  <ConfirmDialog
    open={failure !== null}
    title={failure?.retry ? 'Sin conexión' : failure?.signIn ? 'Inicia sesión de nuevo' : 'Algo salió mal'}
    message={failure?.message ?? ''}
    confirmLabel={failure?.retry ? 'Reintentar' : failure?.repair ? 'Reparar' : failure?.signIn ? 'Ir a Perfiles' : 'Entendido'}
    cancelLabel={failure?.retry || failure?.repair || failure?.signIn ? 'Cerrar' : ''}
    onconfirm={onFailureConfirm}
    oncancel={() => instance && sessions.dismiss(instance.id)}
  />

  <section class="console-panel panel" aria-label="Consola de la instancia">
    <div class="console-heading">
      <div>
        <span class="terminal-symbol" aria-hidden="true">›_</span>
        <h3>Consola de la instancia</h3>
        <span class="tiny-label">SALIDA DEL JUEGO</span>
      </div>
      <button class="text-link" disabled={lines.length === 0} onclick={() => sessions.clearLog(instance.id)}>Limpiar consola</button>
    </div>

    <div class="console-tools">
      <input aria-label="Buscar en la consola" placeholder="Buscar en la consola…" bind:value={filter} />
      <select aria-label="Tipo de mensajes" bind:value={level}>
        <option value="all">Todos</option>
        <option value="WARN">Avisos</option>
        <option value="ERROR">Errores</option>
      </select>
      <span>{visible.length} {visible.length === 1 ? 'línea' : 'líneas'}</span>
    </div>

    <div class="console-output" role="log" aria-label="Salida del juego">
      {#if visible.length}
        {#each visible as line, i (i)}
          {@const kind = levelOf(line.text)}
          <div class="log-line" class:warning={kind === 'WARN'} class:error={kind === 'ERROR'}>
            <span>{String(i + 1).padStart(2, '0')}</span>
            <code>{line.text}</code>
          </div>
        {/each}
      {:else}
        <div class="console-empty">
          <span aria-hidden="true">›_</span>
          <strong>{emptyHeading}</strong>
          <p>Lo que escriba Minecraft aparecerá aquí al pulsar JUGAR. Es útil si algo falla y necesitas pedir ayuda.</p>
        </div>
      {/if}
    </div>

    <div class="console-status">
      <span>
        <span class="status-dot" class:warn={!running}></span>
        {running ? 'Minecraft está abierto' : 'Minecraft no está abierto'}
      </span>
      <span>Solo lectura</span>
    </div>
  </section>
{:else}
  <section class="session-card panel">
    <div class="session-empty">
      <div>
        <h2>Crea tu primera instancia</h2>
        <p>Una instancia es tu mundo de Minecraft con su propia versión. Elige la que quieras y juega.</p>
      </div>
      <Button variant="primary" large onclick={() => nav.create()}>Crear instancia</Button>
    </div>
  </section>
{/if}
