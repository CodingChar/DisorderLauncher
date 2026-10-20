<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from '../../lib/api'
  import McButton from '../../lib/mc/McButton.svelte'
  import McField from '../../lib/mc/McField.svelte'
  import McLabel from '../../lib/mc/McLabel.svelte'
  import McList from '../../lib/mc/McList.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import { useGui } from '../../lib/mc/gui'
  import type { RowSpec } from '../../lib/mc/listTypes'
  import { bandWidth } from '../../lib/mc/pos'
  import type { JvmCheck, JvmTweakInfo, Optimization } from '../../lib/types'

  interface Props {
    instanceId: string
    /** Nivel de rendimiento elegido (aunque aun no se haya guardado). */
    level: Optimization
    /** Ids de los ajustes activados a mano. */
    tweaks: string[]
    /** Argumentos propios, separados por espacios. */
    args: string
    disabled?: boolean
    onback: () => void
  }

  let { instanceId, level, tweaks = $bindable(), args = $bindable(), disabled = false, onback }: Props = $props()

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  let check = $state<JvmCheck | null>(null)
  let error = $state<string | null>(null)
  let busy = $state(false)
  let selectedId = $state<string | null>(null)
  // Solo vale la respuesta de la ultima comprobacion pedida.
  let ticket = 0

  const RISK: Record<string, { text: string; color: string }> = {
    safe: { text: 'Segura', color: '#55ff55' },
    tradeoff: { text: 'Tiene contrapartida', color: '#ffff55' },
    experimental: { text: 'Experimental', color: '#ffaa00' },
  }

  async function run(): Promise<boolean> {
    const mine = ++ticket
    busy = true
    try {
      const result = await api.instancesJvmCheck(instanceId, level, tweaks, args)
      if (mine !== ticket) return false
      check = result
      error = null
      return true
    } catch (e) {
      if (mine !== ticket) return false
      error = (e as Error).message ?? String(e)
      return false
    } finally {
      if (mine === ticket) busy = false
    }
  }

  onMount(() => {
    void run()
  })

  const selected = $derived(check?.tweaks.find((t) => t.id === selectedId) ?? null)
  const active = (t: JvmTweakInfo) => tweaks.includes(t.id)

  function row(t: JvmTweakInfo): RowSpec {
    const on = active(t)
    const byLevel = t.fromLevel && !on
    const mark = on ? '[x]' : byLevel ? '[=]' : '[ ]'
    const status = t.unavailable ? `No disponible: ${t.unavailable}` : byLevel ? 'Ya lo activa el nivel de rendimiento' : RISK[t.risk]?.text ?? ''
    return {
      lines: [
        { text: `${mark} ${t.label}`, color: t.unavailable ? '#808080' : on || byLevel ? '#55ff55' : '#ffffff' },
        { text: status, color: t.unavailable ? '#ff5555' : (RISK[t.risk]?.color ?? '#a0a0a0') },
      ],
    }
  }

  function toggle() {
    if (!selected || disabled || selected.unavailable) return
    tweaks = active(selected) ? tweaks.filter((id) => id !== selected.id) : [...tweaks, selected.id]
    void run()
  }

  async function done() {
    // No se sale con argumentos que Java o el launcher rechazan: se corrigen antes.
    if (await run()) onback()
  }

  const summary = $derived.by(() => {
    if (error) return { text: error, color: '#ff5555' }
    if (!check) return { text: 'Comprobando…', color: '#a0a0a0' }
    const count = check.flags.length
    const dropped = check.skipped.filter((s) => s.reason.startsWith('Java no la acepta'))
    if (dropped.length) return { text: `Java no acepta: ${dropped.map((s) => s.id).join(', ')}. Se omitirá al jugar.`, color: '#ffaa00' }
    const how = check.verified ? `comprobado con Java ${check.javaMajor}` : `Java ${check.javaMajor} aún no instalado: se comprobará al jugar`
    return { text: `${count === 0 ? 'Sin opciones extra' : `${count} opciones de Java`} · ${how}`, color: check.verified ? '#55ff55' : '#a0a0a0' }
  })

  const detail = $derived(selected ? `${selected.hint}${selected.risk === 'experimental' ? ' (opción experimental de Java)' : ''}` : 'Elige un ajuste para ver qué hace. Los de nivel «Normal» no cambian nada si no los activas.')
</script>

<McScreen title="Ajustes de Java">
  <McList
    items={check?.tweaks ?? []}
    getId={(t) => t.id}
    {row}
    bind:selectedId
    x={-(bw >> 1)}
    y={22}
    w={bw}
    h={Math.max(40, gui.H - 170)}
    rowHeight={24}
    loading={!check && !error}
    onactivate={toggle}
    empty="No hay ajustes disponibles."
    ariaLabel="Ajustes de Java"
  />
  <McText text={detail} x={-155} y={gui.H - 146} w={310} color="#a0a0a0" maxLines={2} />
  <McLabel x={-155} y={gui.H - 120} color="#a0a0a0" text="Argumentos de Java propios (opcional)" />
  <McField
    x={-155}
    y={gui.H - 110}
    w={310}
    maxlength={400}
    bind:value={args}
    {disabled}
    placeholder="-XX:MaxGCPauseMillis=100"
    label="Argumentos de Java propios"
    onenter={() => void run()}
  />
  <McText text={summary.text} x={-155} y={gui.H - 84} w={310} color={summary.color} maxLines={2} role={error ? 'alert' : 'status'} />
  <McButton anchor="bottom" x={-155} y={-28} w={100} label={selected && active(selected) ? 'Quitar' : 'Activar'} disabled={disabled || !selected || !!selected?.unavailable} onclick={toggle} />
  <McButton anchor="bottom" x={-50} y={-28} w={100} label={busy ? 'Probando…' : 'Comprobar'} disabled={busy} onclick={() => void run()} />
  <McButton anchor="bottom" x={55} y={-28} w={100} label="Hecho" onclick={() => void done()} />
</McScreen>
