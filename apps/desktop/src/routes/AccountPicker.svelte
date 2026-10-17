<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import ProfileBadge from '../lib/components/ProfileBadge.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'
  import type { AccountRecord, AuthState } from '../lib/types'

  let name = $state('')
  let error = $state<string | null>(null)
  let busy = $state(false)
  let toRemove = $state<AccountRecord | null>(null)

  onMount(() => {
    accounts.loadMicrosoft()
    accounts.refreshStates()
  })

  async function add() {
    if (busy) return
    busy = true
    error = null
    try {
      await accounts.createLocal(name)
      name = ''
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }

  async function use(id: string) {
    error = null
    try {
      await accounts.setDefault(id)
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function connect() {
    error = null
    try {
      await accounts.beginLogin()
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function confirmRemove() {
    const target = toRemove
    toRemove = null
    if (!target) return
    error = null
    try {
      await accounts.remove(target.id)
    } catch (e) {
      error = (e as Error).message
    }
  }

  function stateText(state: AuthState | undefined): string | null {
    if (!state || state === 'LocalReady') return null
    if (state === 'MicrosoftAuthenticated') return 'Sesión activa'
    if (state === 'MicrosoftExpired' || state === 'MicrosoftDisconnected') return 'Necesita volver a iniciar sesión'
    if (state === 'MicrosoftOfflineCachedIdentity') return 'Sin conexión: se comprobará cuando vuelva internet'
    if (typeof state === 'object') return 'No se pudo comprobar la cuenta'
    return null
  }

  const needsSignIn = (id: string) => {
    const s = accounts.states[id]
    return s === 'MicrosoftExpired' || s === 'MicrosoftDisconnected'
  }
</script>

<div class="page">
  <header>
    <small>PERFILES</small>
    <h1>¿Quién juega?</h1>
  </header>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <ul class="list">
    {#each accounts.list as account (account.id)}
      {@const status = stateText(accounts.states[account.id])}
      <li>
        <Card>
          <div class="row">
            <div class="who">
              <ProfileBadge {account} />
              {#if status}<small class:warn={needsSignIn(account.id)}>{status}</small>{/if}
            </div>
            <div class="actions">
              {#if account.provider === 'microsoft' && needsSignIn(account.id) && accounts.microsoft.enabled}
                <Button variant="primary" disabled={accounts.microsoft.loginInProgress} onclick={connect}>Volver a iniciar sesión</Button>
              {/if}
              {#if account.id === accounts.defaultId}
                <span class="current">En uso</span>
              {:else}
                <Button onclick={() => use(account.id)}>Usar</Button>
              {/if}
              <Button variant="ghost" onclick={() => (toRemove = account)}>Quitar</Button>
            </div>
          </div>
        </Card>
      </li>
    {/each}
  </ul>

  <Card>
    <form
      onsubmit={(e) => {
        e.preventDefault()
        add()
      }}
    >
      <h3>Añadir perfil sin conexión</h3>
      <p class="soon">Offline Gaming: juega sin cuenta, con el nombre que elijas.</p>
      <Input label="Nombre" bind:value={name} {error} maxlength={16} placeholder="Alex" />
      <div>
        <Button type="submit" disabled={busy || name.trim().length === 0}>Crear perfil</Button>
      </div>
    </form>
  </Card>

  <Card>
    <div class="ms">
      <div>
        <h3>Microsoft</h3>
        {#if !accounts.microsoft.enabled}
          <p class="soon">
            El inicio de sesión con Microsoft no está activado en esta versión. Offline Gaming funciona con normalidad.
          </p>
          <details>
            <summary>Opciones avanzadas</summary>
            <p class="soon">Si tienes una aplicación registrada en Azure, indica su identificador en Ajustes › Avanzado.</p>
          </details>
        {:else if accounts.microsoft.loginInProgress}
          <p class="soon" role="status">Completa el inicio de sesión en tu navegador. Esta ventana se actualizará sola.</p>
        {:else}
          <p class="soon">Conecta tu cuenta para jugar con tu perfil oficial. Se abrirá tu navegador.</p>
        {/if}
        {#if accounts.loginMessage}<p class="soon" role="status"><strong>{accounts.loginMessage}</strong></p>{/if}
      </div>
      {#if accounts.microsoft.loginInProgress}
        <Button onclick={() => accounts.cancelLogin()}>Cancelar</Button>
      {:else}
        <Button disabled={!accounts.microsoft.enabled} onclick={connect}>Conectar cuenta</Button>
      {/if}
    </div>
  </Card>

  <ConfirmDialog
    open={toRemove !== null}
    title="¿Quitar este perfil?"
    message={toRemove
      ? toRemove.provider === 'microsoft'
        ? `Se cerrará la sesión de «${toRemove.label}» en este equipo. Tus instancias no se borran.`
        : `Se quitará el perfil «${toRemove.label}». Tus instancias no se borran.`
      : ''}
    confirmLabel="Quitar"
    onconfirm={confirmRemove}
    oncancel={() => (toRemove = null)}
  />
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    max-width: 640px;
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

  h3 {
    font-size: 18px;
    margin-bottom: 6px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .row,
  .ms {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .who {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .who small {
    color: var(--text-secondary);
  }

  .who small.warn {
    color: var(--text-primary);
    font-weight: 600;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .current {
    padding: 8px 14px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    font-size: 12px;
    font-weight: 600;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .soon {
    margin: 0 0 6px;
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 1.5;
    max-width: 440px;
  }

  details {
    color: var(--text-secondary);
    font-size: 13px;
  }

  summary {
    cursor: pointer;
    min-height: 32px;
  }

  .error {
    margin: 0;
    font-weight: 600;
  }
</style>
