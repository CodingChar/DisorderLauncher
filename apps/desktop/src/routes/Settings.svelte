<script lang="ts">
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McSlider from '../lib/mc/McSlider.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import { setSfxVolume, sfxVolume } from '../lib/audio/sfx'
  import { panoramaMoves, setPanoramaMoves } from '../lib/mc/motion'
  import { FEATURES } from '../lib/features'
  import { nav } from '../lib/stores/nav.svelte'
  import { settings } from '../lib/stores/settings.svelte'
  import type { DownloadProfile } from '../lib/types'

  const gui = useGui()

  const speeds: { value: DownloadProfile; label: string; hint: string }[] = [
    { value: 'eco', label: 'Económica', hint: 'Usa menos internet; las descargas tardan más.' },
    { value: 'normal', label: 'Normal', hint: 'Equilibrada (recomendado).' },
    { value: 'performance', label: 'Rápida', hint: 'Descarga lo más rápido posible.' },
  ]

  let sub = $state<'main' | 'advanced'>('main')
  let speed = $state<DownloadProfile>(settings.value.downloadProfile)
  let javaPath = $state(settings.value.javaPath ?? '')
  let msClientId = $state(settings.value.microsoftClientId ?? '')
  let volume = $state(Math.round(sfxVolume() * 100))
  let background = $state<'moving' | 'still'>(panoramaMoves() ? 'moving' : 'still')
  const backgrounds = [
    { value: 'moving' as const, label: 'En movimiento' },
    { value: 'still' as const, label: 'Quieto' },
  ]
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)

  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  const speedHint = $derived(speeds.find((s) => s.value === speed)?.hint ?? '')

  async function save() {
    if (saving) return
    saving = true
    error = null
    notice = null
    try {
      await settings.save({
        ...settings.value,
        theme: 'dark',
        downloadProfile: speed,
        themeId: null,
        microsoftClientId: msClientId.trim() || null,
        javaPath: javaPath.trim() || null,
      })
      notice = 'Preferencias guardadas.'
    } catch (e) {
      error = (e as Error).message
    } finally {
      saving = false
    }
  }

</script>

{#if sub === 'advanced'}
  <McScreen title="Opciones avanzadas">
    <McLabel x={-155} y={40} color="#a0a0a0" text="ID de la aplicación de Microsoft" />
    <McField x={-155} y={50} w={310} maxlength={64} bind:value={msClientId} placeholder="00000000-0000-0000-0000-000000000000" label="ID de la aplicación de Microsoft" hint="Solo si tienes tu propia aplicación registrada en Azure. Déjalo vacío para no usar Microsoft." />
    <McButton anchor="bottom" y={-28} label="Hecho" onclick={() => (sub = 'main')} />
  </McScreen>
{:else}
  <McScreen title="Ajustes">
    <form
      class="mc-form"
      onsubmit={(e) => {
        e.preventDefault()
        save()
      }}
    >
      <McLabel center y={30} text="Apariencia clásica de Minecraft" />
      <McSlider y={44} bind:value={volume} min={0} max={100} step={5} format={(v) => `Volumen de sonidos: ${v}%`} label="Volumen de sonidos" onchange={(v) => setSfxVolume(v / 100)} />
      <McCycle y={68} label="Fondo del menú" options={backgrounds} bind:value={background} onchange={(v) => setPanoramaMoves(v === 'moving')} />
      <McCycle y={92} label="Descargas" options={speeds} bind:value={speed} />
      <McText text={speedHint} x={-155} y={116} w={310} align="center" color="#808080" maxLines={1} />

      <McLabel x={-155} y={132} color="#a0a0a0" text="Java propio (opcional)" />
      <McField x={-155} y={142} w={310} maxlength={260} bind:value={javaPath} placeholder="Déjalo vacío para que se gestione solo" label="Java propio (opcional)" />
      <McText text="Solo se usa si es compatible con la versión de Minecraft; si no, el launcher elige uno adecuado." x={-155} y={166} w={310} color="#808080" maxLines={2} />

      <McLabel x={-155} y={gui.H - 52} color="#707070" max={310} text={`Carpeta de datos: ${settings.dataDir || '—'}`} />
      {#if FEATURES.microsoftSettings}
        <McButton x={-155} y={gui.H - 78} w={150} label="Avanzado..." ariaLabel="Opciones avanzadas" onclick={() => (sub = 'advanced')} />
      {/if}

      {#if error}
        <McText text={error} x={-155} y={gui.H - 50} w={310} color="#ff5555" maxLines={2} role="alert" />
      {:else if notice}
        <McText text={notice} x={-155} y={gui.H - 50} w={310} color="#55ff55" maxLines={2} role="status" />
      {/if}

      <McButton anchor="bottom" x={-154} y={-28} w={150} type="submit" label={saving ? 'Guardando…' : 'Guardar'} disabled={saving} />
      <McButton anchor="bottom" x={4} y={-28} w={150} label="Atrás" onclick={() => nav.go('menu')} />
    </form>
  </McScreen>
{/if}
