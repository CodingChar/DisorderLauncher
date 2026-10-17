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
  <div class="card panel">
    <div class="brand-line" aria-hidden="true">disorder<span>LAUNCHER</span></div>
    <span class="eyebrow">PRIMER INICIO</span>
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

  .card {
    width: min(460px, 100%);
    padding: 36px;
    border-radius: var(--r-xl);
    box-shadow: var(--shadow);
  }

  .brand-line {
    font-family: var(--font-display);
    font-size: 25px;
    font-weight: 700;
    letter-spacing: -1.3px;
    line-height: 1;
    margin-bottom: 30px;
  }

  .brand-line span {
    display: block;
    font-family: var(--font-body);
    font-weight: 400;
    letter-spacing: 4.8px;
    font-size: 9px;
    margin-top: 6px;
  }

  h1 {
    font-size: 34px;
    margin: 10px 0 8px;
    letter-spacing: -1.25px;
    font-weight: 500;
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
