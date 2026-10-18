<script lang="ts">
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McCycle from '../lib/mc/McCycle.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McField from '../lib/mc/McField.svelte'
  import McLabel from '../lib/mc/McLabel.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { FEATURES } from '../lib/features'
  import { nav } from '../lib/stores/nav.svelte'
  import { settings } from '../lib/stores/settings.svelte'
  import type { DownloadProfile, Theme, ThemeEntry } from '../lib/types'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  const DEFAULT_IDS = ['disorderlaucher.default.dark', 'disorderlaucher.default.light']
  const themes: { value: Theme; label: string }[] = [
    { value: 'system', label: 'Sistema' },
    { value: 'light', label: 'Claro' },
    { value: 'dark', label: 'Oscuro' },
  ]
  const speeds: { value: DownloadProfile; label: string; hint: string }[] = [
    { value: 'eco', label: 'Económica', hint: 'Usa menos internet; las descargas tardan más.' },
    { value: 'normal', label: 'Normal', hint: 'Equilibrada (recomendado).' },
    { value: 'performance', label: 'Rápida', hint: 'Descarga lo más rápido posible.' },
  ]

  let sub = $state<'main' | 'themes' | 'advanced'>('main')
  let theme = $state<Theme>(settings.value.theme)
  let speed = $state<DownloadProfile>(settings.value.downloadProfile)
  let javaPath = $state(settings.value.javaPath ?? '')
  let themeId = $state(settings.value.themeId ?? '')
  let msClientId = $state(settings.value.microsoftClientId ?? '')
  let pickId = $state('')
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)
  let confirmRemove = $state(false)

  // Mantener el formulario al dia si el tema cambia desde fuera de esta pantalla.
  $effect(() => {
    themeId = settings.value.themeId ?? ''
  })

  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  // Los temas claro y oscuro se eligen con «Apariencia»; aqui van los demas (incluido el azul y los importados).
  const extraThemes = $derived(settings.themes.filter((t) => !DEFAULT_IDS.includes(t.id)))
  const themeRemovable = $derived(!!pickId && settings.themes.some((t) => t.id === pickId && !t.builtin))
  const themeItems = $derived<{ id: string; name: string; theme?: ThemeEntry }[]>([
    { id: '', name: 'Ninguno (usar la apariencia)' },
    ...extraThemes.map((t) => ({ id: t.id, name: t.name, theme: t })),
  ])
  const currentThemeName = $derived(themeItems.find((t) => t.id === themeId)?.name ?? 'Ninguno')
  const speedHint = $derived(speeds.find((s) => s.value === speed)?.hint ?? '')

  function openThemes() {
    pickId = themeId
    error = null
    notice = null
    sub = 'themes'
  }

  function useTheme() {
    themeId = pickId
    notice = 'Tema elegido. Pulsa Guardar para aplicarlo.'
    sub = 'main'
  }

  async function importTheme() {
    error = null
    notice = null
    try {
      const imported = await settings.importTheme()
      if (imported) {
        pickId = imported.id
        notice = `Tema «${imported.name}» importado. Elígelo y pulsa Usar tema.`
      }
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function removeTheme() {
    confirmRemove = false
    if (!pickId) return
    error = null
    try {
      const removed = pickId
      await settings.removeTheme(removed)
      if (themeId === removed) themeId = ''
      pickId = ''
      notice = 'Tema quitado.'
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function save() {
    if (saving) return
    saving = true
    error = null
    notice = null
    try {
      await settings.save({
        ...settings.value,
        theme,
        downloadProfile: speed,
        themeId: themeId || null,
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

  function themeRow(t: { id: string; name: string; theme?: ThemeEntry }): RowSpec {
    const active = t.id === themeId
    return {
      lines: [
        { text: active ? `${t.name}  (En uso)` : t.name, color: active ? '#55ff55' : '#ffffff' },
        { text: t.theme ? (t.theme.builtin ? 'Incluido' : 'Importado') : 'Solo cambia la apariencia' },
      ],
    }
  }
</script>

{#if sub === 'themes'}
  <McScreen title="Tema especial">
    <McList items={themeItems} getId={(t) => t.id} row={themeRow} bind:selectedId={pickId} onactivate={useTheme} x={-(bw >> 1)} y={22} w={bw} h={gui.H - 92} rowHeight={24} ariaLabel="Temas" />
    {#if error}
      <McText text={error} x={-155} y={gui.H - 68} w={310} align="center" color="#ff5555" role="alert" />
    {:else if notice}
      <McText text={notice} x={-155} y={gui.H - 68} w={310} align="center" color="#55ff55" role="status" />
    {/if}
    <McButton anchor="bottom" x={-154} y={-52} w={150} label="Usar tema" onclick={useTheme} />
    <McButton anchor="bottom" x={4} y={-52} w={150} label="Importar tema..." onclick={importTheme} />
    <McButton anchor="bottom" x={-154} y={-28} w={150} label="Quitar este tema" disabled={!themeRemovable} onclick={() => (confirmRemove = true)} />
    <McButton anchor="bottom" x={4} y={-28} w={150} label="Hecho" onclick={() => (sub = 'main')} />

    {#if confirmRemove}
      <McDialog
        title="¿Quitar este tema?"
        message="Se borrará el tema importado. Un tema solo cambia el aspecto; nunca ejecuta código."
        buttons={[
          { label: 'Cancelar', onclick: () => (confirmRemove = false) },
          { label: 'Quitar', onclick: removeTheme },
        ]}
        cancelIndex={0}
        onclose={() => (confirmRemove = false)}
      />
    {/if}
  </McScreen>
{:else if sub === 'advanced'}
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
      <McCycle y={22} label="Apariencia" options={themes} bind:value={theme} />
      <McButton y={46} label="Tema especial..." ariaLabel="Tema especial: {currentThemeName}. Pulsa para cambiar" onclick={openThemes} />
      <McCycle y={70} label="Descargas" options={speeds} bind:value={speed} />
      <McText text={speedHint} x={-155} y={94} w={310} align="center" color="#808080" maxLines={1} />

      <McLabel x={-155} y={112} color="#a0a0a0" text="Java propio (opcional)" />
      <McField x={-155} y={122} w={310} maxlength={260} bind:value={javaPath} placeholder="Déjalo vacío para que se gestione solo" label="Java propio (opcional)" />
      <McText text="Solo se usa si es compatible con la versión de Minecraft; si no, el launcher elige uno adecuado." x={-155} y={146} w={310} color="#808080" maxLines={2} />

      <McLabel x={-155} y={170} color="#707070" max={310} text={`Carpeta de datos: ${settings.dataDir || '—'}`} />
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
