<script lang="ts">
  import type { Session } from './types'
  import Avatar from './Avatar.svelte'
  import ItemLogo from './ItemLogo.svelte'

  let {
    session,
    clearedSession,
    onClearConsole,
    goInstances,
  }: {
    session: Session | null
    clearedSession: Session | null
    onClearConsole: (session: Session | null) => void
    goInstances: () => void
  } = $props()

  let filter = $state('')
  let level = $state('Todos')

  const logs = $derived(
    !session || clearedSession === session ? [] : session.logs,
  )
  const visible = $derived(
    logs.filter(
      (log) =>
        log.toLowerCase().includes(filter.toLowerCase()) &&
        (level === 'Todos' || log.includes(level)),
    ),
  )
  // `clearedSession === session` también sería true cuando ambos son null, así que
  // hace falta comprobar que hay sesión para hablar de una consola limpiada.
  const isCleared = $derived(!!session && clearedSession === session)
  const emptyHeading = $derived(
    filter || level !== 'Todos'
      ? 'Sin resultados para este filtro.'
      : isCleared
        ? 'Consola limpiada.'
        : 'Esperando una sesión…',
  )
</script>

<div class="session-monitor">
  <section class="session-card panel">
    <div class="session-profile">
      <Avatar />
      <div>
        <strong>{session?.profile ?? 'Sin perfil en uso'}</strong>
        <span>SKIN DE LA SESIÓN</span>
      </div>
      <span class="pill">
        <span class="status-dot"></span>
        {session ? 'EN EJECUCIÓN · DEMO' : 'SIN SESIÓN ACTIVA'}
      </span>
    </div>

    {#if session}
      <div class="session-identity">
        <ItemLogo type={session.instance.type} />
        <div>
          <h2>{session.instance.name}</h2>
          <p>
            Minecraft {session.instance.version} <span>·</span>
            {session.instance.loader} <span>·</span>
            {session.instance.mods} mods
          </p>
        </div>
        <span class="session-started">Inicio: {session.started}</span>
      </div>
      <div class="session-details">
        <span>
          RAM <strong>{session.instance.minRam}–{session.instance.maxRam} GB</strong>
        </span>
        <span>
          Java <strong>{session.instance.java}</strong>
        </span>
        <span>
          Compatibilidad <strong>{session.instance.compatibility}</strong>
        </span>
        <span>
          Ventana <strong>
            {session.instance.fullscreen
              ? 'Pantalla completa'
              : `${session.instance.width} × ${session.instance.height}`}
          </strong>
        </span>
      </div>
    {:else}
      <div class="session-empty">
        <div>
          <h2>No hay instancias en ejecución.</h2>
          <p>
            Inicia una instancia desde tu biblioteca para ver aquí sus detalles y su
            consola.
          </p>
        </div>
        <button class="secondary" onclick={goInstances}>
          Ir a Mis instancias →
        </button>
      </div>
    {/if}
  </section>

  <section class="console-panel panel">
    <div class="console-heading">
      <div>
        <span class="terminal-symbol">›_</span>
        <h3>Consola de la instancia</h3>
        <span class="tiny-label">LOGS SIMULADOS</span>
      </div>
      <button
        class="text-link"
        disabled={!logs.length}
        onclick={() => onClearConsole(session)}
      >
        Limpiar consola
      </button>
    </div>

    <div class="console-tools">
      <input
        aria-label="Buscar en logs"
        placeholder="Buscar en la consola…"
        bind:value={filter}
      />
      <select aria-label="Nivel de logs" bind:value={level}>
        <option>Todos</option>
        <option>INFO</option>
        <option>WARN</option>
      </select>
      <span>{visible.length} líneas</span>
    </div>

    <div class="console-output" role="log" aria-label="Logs de demostración">
      {#if visible.length}
        {#each visible as line, i (i)}
          <div class="log-line {line.includes('WARN') ? 'warning' : ''}">
            <span>{String(i + 1).padStart(2, '0')}</span>
            <code>{line}</code>
          </div>
        {/each}
      {:else}
        <div class="console-empty">
          <span>›_</span>
          <strong>{emptyHeading}</strong>
          <p>
            {session
              ? 'Los logs de esta demostración aparecerán al iniciar una nueva sesión.'
              : 'La salida de la instancia aparecerá aquí.'}
          </p>
        </div>
      {/if}
    </div>

    <div class="console-status">
      <span>
        <span class="status-dot"></span>
        {session ? 'Sesión de demostración activa' : 'Sin proceso conectado'}
      </span>
      <span>Solo lectura · No ejecuta Minecraft</span>
    </div>
  </section>
</div>