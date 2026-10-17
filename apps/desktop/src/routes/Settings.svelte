<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import Input from '../lib/components/Input.svelte'
  import { settings } from '../lib/stores/settings.svelte'
  import type { DownloadProfile, Theme } from '../lib/types'

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
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let saving = $state(false)

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
  <header>
    <small>PREFERENCIAS</small>
    <h1>Ajustes</h1>
  </header>

  <Card>
    <form
      onsubmit={(e) => {
        e.preventDefault()
        save()
      }}
    >
      <fieldset>
        <legend>Apariencia</legend>
        {#each themes as option (option.value)}
          <label class="radio"><input type="radio" bind:group={theme} value={option.value} /> {option.label}</label>
        {/each}
      </fieldset>

      <fieldset>
        <legend>Velocidad de descarga</legend>
        {#each speeds as option (option.value)}
          <label class="radio">
            <input type="radio" bind:group={speed} value={option.value} />
            <span>{option.label} <small>{option.hint}</small></span>
          </label>
        {/each}
      </fieldset>

      <Input
        label="Java propio (opcional)"
        bind:value={javaPath}
        placeholder="Déjalo vacío para que se gestione solo"
        hint="Solo se usa si es compatible con la versión de Minecraft; si no, el launcher elige uno adecuado."
      />

      <div class="readonly">
        <span class="label">Carpeta de datos</span>
        <code>{settings.dataDir || '—'}</code>
      </div>

      {#if error}<p class="error" role="alert">{error}</p>{/if}
      {#if notice}<p class="notice" role="status">{notice}</p>{/if}

      <div><Button variant="primary" type="submit" disabled={saving}>{saving ? 'Guardando…' : 'Guardar'}</Button></div>
    </form>
  </Card>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 560px;
  }

  header small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.18em;
    margin-bottom: 5px;
  }

  h1 {
    font-size: 34px;
    line-height: 0.95;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  fieldset {
    border: 1px solid var(--border);
    border-radius: var(--r-md);
    padding: 12px 16px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  legend {
    padding: 0 6px;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  .radio {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
  }

  .radio input {
    width: 20px;
    height: 20px;
    accent-color: var(--action);
    flex: none;
  }

  .radio small {
    display: block;
    color: var(--text-secondary);
  }

  .readonly {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--text-secondary);
  }

  code {
    font-size: 12px;
    padding: 10px 12px;
    border-radius: var(--r-sm);
    background: var(--surface-alt);
    overflow-wrap: anywhere;
  }

  .error {
    margin: 0;
    font-weight: 600;
  }

  .notice {
    margin: 0;
    color: var(--text-secondary);
  }
</style>
