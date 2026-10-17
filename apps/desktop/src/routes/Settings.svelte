<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import Card from '../lib/components/Card.svelte'
  import Input from '../lib/components/Input.svelte'
  import Select from '../lib/components/Select.svelte'
  import { settings } from '../lib/stores/settings.svelte'
  import type { DownloadProfile, Theme } from '../lib/types'

  const DEFAULT_IDS = ['disorderlaucher.default.dark', 'disorderlaucher.default.light']
  const themes: { value: Theme; label: string }[] = [
    { value: 'system', label: 'Igual que el sistema' },
    { value: 'dark', label: 'Oscuro' },
    { value: 'light', label: 'Claro' },
  ]
  const speeds: { value: DownloadProfile; label: string; hint: string }[] = [
    { value: 'eco', label: 'Económica', hint: 'Usa menos internet; las descargas tardan más.' },
    { value: 'normal', label: 'Normal', hint: 'Equilibrada (recomendado).' },
    { value: 'performance', label: 'Rápida', hint: 'Descarga lo más rápido posible.' },
  ]

  let theme = $state<Theme>(settings.value.theme)
  let speed = $state<DownloadProfile>(settings.value.downloadProfile)
  let javaPath = $state(settings.value.javaPath ?? '')
  let themeId = $state(settings.value.themeId ?? '')
  let msClientId = $state(settings.value.microsoftClientId ?? '')

  // El boton de la luna de la cabecera tambien cambia el tema: mantener el formulario al dia.
  $effect(() => {
    themeId = settings.value.themeId ?? ''
  })
  // Los temas claro y oscuro se eligen arriba; aqui van los demas (incluido el azul y los importados).
  const extraThemes = $derived(settings.themes.filter((t) => !DEFAULT_IDS.includes(t.id)))
  const themeRemovable = $derived(!!themeId && settings.themes.some((t) => t.id === themeId && !t.builtin))
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)

  async function importTheme() {
    error = null
    notice = null
    try {
      const theme = await settings.importTheme()
      if (theme) {
        themeId = theme.id
        notice = `Tema «${theme.name}» importado. Pulsa Guardar para usarlo.`
      }
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function removeTheme() {
    if (!themeId) return
    error = null
    try {
      await settings.removeTheme(themeId)
      themeId = ''
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
</script>

<div class="page">
  <PageHeading eyebrow="PREFERENCIAS" title="Ajustes" description="Apariencia, descargas y Java del launcher." />

  <section class="settings-panel panel">
    <form
      class="form-stack"
      onsubmit={(e) => {
        e.preventDefault()
        save()
      }}
    >
      <fieldset class="settings-block">
        <legend>Apariencia</legend>
        {#each themes as option (option.value)}
          <label class="check-row"><input type="radio" bind:group={theme} value={option.value} /> {option.label}</label>
        {/each}
      </fieldset>

      <fieldset class="settings-block">
        <legend>Tema especial</legend>
        <Select
          label="Tema"
          bind:value={themeId}
          options={[{ value: '', label: 'Ninguno (usar el de arriba)' }, ...extraThemes.map((t) => ({ value: t.id, label: t.name }))]}
        />
        <div class="form-actions">
          <Button onclick={importTheme}>Importar tema…</Button>
          <Button variant="ghost" disabled={!themeRemovable} onclick={removeTheme}>Quitar este tema</Button>
        </div>
        <small class="field-help">Un tema solo cambia colores, formas y tipografía; nunca ejecuta código. El botón de la luna de la cabecera activa el tema azul oscuro.</small>
      </fieldset>

      <fieldset class="settings-block">
        <legend>Velocidad de descarga</legend>
        {#each speeds as option (option.value)}
          <label class="check-row">
            <input type="radio" bind:group={speed} value={option.value} />
            <span>{option.label} <small>{option.hint}</small></span>
          </label>
        {/each}
      </fieldset>

      <div class="settings-block">
        <Input
          label="Java propio (opcional)"
          bind:value={javaPath}
          placeholder="Déjalo vacío para que se gestione solo"
          hint="Solo se usa si es compatible con la versión de Minecraft; si no, el launcher elige uno adecuado."
        />
      </div>

      <details class="settings-block advanced">
        <summary>Opciones avanzadas</summary>
        <Input
          label="ID de la aplicación de Microsoft"
          bind:value={msClientId}
          placeholder="00000000-0000-0000-0000-000000000000"
          hint="Solo si tienes tu propia aplicación registrada en Azure. Déjalo vacío para no usar Microsoft."
        />
      </details>

      <div class="field">
        <span class="field-label">Carpeta de datos</span>
        <code class="mono-box">{settings.dataDir || '—'}</code>
      </div>

      {#if error}<p class="form-error" role="alert">{error}</p>{/if}
      {#if notice}<p class="form-ok" role="status">{notice}</p>{/if}

      <div><Button variant="primary" type="submit" disabled={saving}>{saving ? 'Guardando…' : 'Guardar'}</Button></div>
    </form>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 800px;
  }

  .advanced summary {
    min-height: 40px;
    display: flex;
    align-items: center;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
  }

  .advanced[open] summary {
    margin-bottom: 14px;
  }
</style>
