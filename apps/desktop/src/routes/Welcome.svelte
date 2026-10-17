<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Input from '../lib/components/Input.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'

  let name = $state('')
  let error = $state<string | null>(null)
  let busy = $state(false)

  async function create() {
    if (busy) return
    busy = true
    error = null
    try {
      await accounts.createLocal(name)
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }
</script>

<div class="welcome">
  <div class="panel">
    <span class="kicker">PRIMER INICIO</span>
    <h1>Bienvenido</h1>
    <p>¿Cómo quieres llamarte en el juego?</p>

    <form
      onsubmit={(e) => {
        e.preventDefault()
        create()
      }}
    >
      <Input
        label="Tu nombre"
        bind:value={name}
        {error}
        maxlength={16}
        autofocus
        hint="Entre 3 y 16 caracteres: letras, números o guion bajo (_)."
        placeholder="David"
      />
      <Button variant="primary" type="submit" disabled={busy || name.trim().length === 0}>
        {busy ? 'Creando…' : 'Crear perfil'}
      </Button>
    </form>
  </div>
</div>

<style>
  .welcome {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .panel {
    width: min(460px, 100%);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 24px 0 24px 0;
    padding: 36px;
    box-shadow: var(--shadow);
  }

  .kicker {
    display: inline-block;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: 10px;
    letter-spacing: 0.14em;
    color: var(--text-secondary);
  }

  h1 {
    font-size: 40px;
    margin: 18px 0 8px;
    letter-spacing: -0.06em;
  }

  p {
    margin: 0 0 24px;
    color: var(--text-secondary);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
</style>
