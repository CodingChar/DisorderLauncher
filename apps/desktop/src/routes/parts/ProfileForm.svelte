<script lang="ts">
  import McButton from '../../lib/mc/McButton.svelte'
  import McField from '../../lib/mc/McField.svelte'
  import McLabel from '../../lib/mc/McLabel.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import { accounts } from '../../lib/stores/accounts.svelte'

  interface Props {
    title: string
    /** Frase bajo el titulo (pantalla de bienvenida). */
    intro?: string
    /** Ayuda bajo el campo. */
    hint?: string
    /** Con `oncancel` aparece el boton «Cancelar» (la bienvenida no lo tiene). */
    oncancel?: () => void
    /** Se llama tras crear el perfil. */
    ondone?: () => void
  }

  let { title, intro = '', hint = '', oncancel, ondone }: Props = $props()

  let name = $state('')
  let error = $state<string | null>(null)
  let busy = $state(false)

  async function create() {
    if (busy || name.trim().length === 0) return
    busy = true
    error = null
    try {
      await accounts.createLocal(name)
      name = ''
      ondone?.()
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }
</script>

<McScreen {title}>
  <form
    class="mc-form"
    onsubmit={(e) => {
      e.preventDefault()
      create()
    }}
  >
    {#if intro}<McText text={intro} x={-155} y={28} w={310} align="center" color="#a0a0a0" maxLines={2} />{/if}
    <McLabel x={-100} y={52} color="#a0a0a0" text="Nombre" />
    <McField x={-100} y={62} w={200} maxlength={16} bind:value={name} {error} {hint} disabled={busy} autofocus placeholder="Alex" label="Nombre del perfil" />

    {#if oncancel}
      <McButton anchor="bottom" x={-100} y={-28} w={98} type="submit" label={busy ? 'Creando…' : 'Crear perfil'} disabled={busy || name.trim().length === 0} />
      <McButton anchor="bottom" x={2} y={-28} w={98} label="Cancelar" onclick={oncancel} />
    {:else}
      <McButton anchor="bottom" y={-28} type="submit" label={busy ? 'Creando…' : 'Crear perfil'} disabled={busy || name.trim().length === 0} />
    {/if}
  </form>
</McScreen>
