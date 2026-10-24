<script lang="ts">
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McProgress from '../lib/mc/McProgress.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { loaderName, phaseLabel, versionLabel } from '../lib/text'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions, type LogLine } from '../lib/stores/sessions.svelte'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const instance = $derived(instances.selected)
  const session = $derived(instance ? sessions.get(instance.id) : undefined)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)
  const running = $derived(session?.phase === 'running')
  const profile = $derived((instance && accounts.list.find((a) => a.id === instance.profileId)) || accounts.current)

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
        return session.message ?? 'Minecraft está abierto'
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

  const crashed = $derived(session?.phase === 'exited' && !!session.exitCode && session.exitCode !== 0)
  const isProblem = $derived(session?.phase === 'failed' || crashed)

  // Un fallo (o un cierre inesperado) se muestra en un dialogo con la accion adecuada.
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

  function dismissFailure() {
    if (instance) sessions.dismiss(instance.id)
  }

  function onFailureConfirm() {
    if (!instance || !failure) return
    const id = instance.id
    sessions.dismiss(id)
    if (failure.retry) sessions.launch(id)
    else if (failure.repair) sessions.repair(id)
    else if (failure.signIn) nav.go('accounts')
  }

  const failureButtons = $derived.by(() => {
    if (!failure) return []
    const confirm = failure.retry ? 'Reintentar' : failure.repair ? 'Reparar' : failure.signIn ? 'Ir a Perfiles' : 'Entendido'
    const buttons = [{ label: confirm, onclick: onFailureConfirm }]
    if (failure.retry || failure.repair || failure.signIn) buttons.push({ label: 'Cerrar', onclick: dismissFailure })
    return buttons
  })

  const infoLine = $derived(
    instance
      ? [`Minecraft ${versionLabel(instance.minecraftVersion)}`, loaderName(instance.loader), profile ? `Perfil: ${profile.label}` : null, session?.startedAt ? `Inicio ${session.startedAt}` : null]
          .filter(Boolean)
          .join(' · ')
      : '',
  )

  // ------------------------------------------------------------------ consola (salida real del juego)
  let filter = $state('')
  let level = $state<'all' | 'WARN' | 'ERROR'>('all')
  const levels = [
    { value: 'all' as const, label: 'Todos' },
    { value: 'WARN' as const, label: 'Avisos' },
    { value: 'ERROR' as const, label: 'Errores' },
  ]

  const lines = $derived(instance ? sessions.logOf(instance.id) : [])
  const query = $derived(filter.toLowerCase())
  const visible = $derived(
    !query && level === 'all' ? lines : lines.filter((line) => {
      if (query && !line.text.toLowerCase().includes(query)) return false
      if (level === 'all') return true
      return levelOf(line.text) === level
    }),
  )

  function levelOf(text: string): 'WARN' | 'ERROR' | 'INFO' {
    if (/\b(ERROR|FATAL|SEVERE)\b/.test(text)) return 'ERROR'
    if (/\b(WARN|WARNING)\b/.test(text)) return 'WARN'
    return 'INFO'
  }

  function lineRow(line: LogLine): RowSpec {
    const kind = levelOf(line.text)
    const color = line.stream === 'stderr' || kind === 'ERROR' ? '#ff5555' : kind === 'WARN' ? '#ffaa00' : /\bDEBUG\b/.test(line.text) ? '#808080' : '#ffffff'
    return { lines: [{ text: line.text, color }] }
  }

  const emptyText = $derived(
    filter || level !== 'all'
      ? 'Sin resultados para este filtro.'
      : (running || busy ? 'Esperando la salida del juego...' : 'Aún no hay salida del juego.') + ' Lo que escriba Minecraft aparecerá aquí al pulsar Jugar.',
  )
</script>

<McScreen title={instance?.name ?? 'Centro de sesión'}>
  {#if instance}
    <McLabel center y={20} color="#a0a0a0" max={gui.W - 8} text={infoLine} />
    {#if session && (session.phase === 'installing' || session.phase === 'repairing')}
      <McProgress {percent} label={statusText} x={-100} y={46} />
    {:else}
      <McLabel center y={32} color={isProblem ? '#ff5555' : '#ffffff'} max={gui.W - 8} text={statusText} />
    {/if}

    <McField x={-155} y={58} w={190} bind:value={filter} placeholder="Buscar en la consola..." label="Buscar en la consola" />
    <McCycle x={39} y={58} w={116} label="Mostrar" options={levels} bind:value={level} />
    <McList
      items={visible}
      getId={(l) => String(l.id)}
      row={lineRow}
      x={-(bw >> 1)}
      y={82}
      w={bw}
      h={gui.H - 140}
      rowHeight={10}
      selectable={false}
      followEnd
      empty={emptyText}
      ariaLabel="Salida del juego"
    />

    {#if running}
      <McButton anchor="bottom" x={-154} y={-52} w={150} label="Cerrar juego" onclick={() => sessions.stop(instance.id)} />
    {:else}
      <McButton anchor="bottom" x={-154} y={-52} w={150} label={busy ? 'Un momento…' : 'Jugar'} disabled={busy} onclick={() => sessions.launch(instance.id)} />
    {/if}
    <McButton anchor="bottom" x={4} y={-52} w={150} label="Limpiar consola" disabled={lines.length === 0} onclick={() => sessions.clearLog(instance.id)} />
    <McButton anchor="bottom" x={-150} y={-28} w={72} label="Mods" disabled={busy || instance.loader === 'vanilla'} onclick={() => nav.go('mods')} />
    <McButton anchor="bottom" x={-74} y={-28} w={72} label="Configurar" disabled={busy} onclick={() => nav.go('edit')} />
    <McButton anchor="bottom" x={2} y={-28} w={72} label="Reparar" disabled={busy} onclick={() => sessions.repair(instance.id)} />
    <McButton anchor="bottom" x={78} y={-28} w={72} label="Cambiar" ariaLabel="Cambiar instancia" onclick={() => nav.go('instances')} />

    {#if failure}
      <McDialog
        title={failure.retry ? 'Sin conexión' : failure.signIn ? 'Inicia sesión de nuevo' : 'Algo salió mal'}
        message={failure.message}
        error
        buttons={failureButtons}
        cancelIndex={0}
        onclose={dismissFailure}
      />
    {/if}
  {:else}
    <McText text="Crea tu primera instancia" x={-155} y={70} w={310} align="center" />
    <McText text="Una instancia es tu mundo de Minecraft con su propia versión. Elige la que quieras y juega." x={-155} y={88} w={310} align="center" color="#a0a0a0" maxLines={3} />
    <McButton label="Crear instancia" y={130} onclick={() => nav.create()} />
    <McButton anchor="bottom" y={-28} label="Atrás" onclick={() => nav.go('menu')} />
  {/if}
</McScreen>
