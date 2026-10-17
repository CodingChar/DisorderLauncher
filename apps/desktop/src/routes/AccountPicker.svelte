<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import Avatar from '../lib/components/Avatar.svelte'
  import Icon from '../lib/components/Icon.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { providerLabel } from '../lib/text'
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
  <PageHeading eyebrow="DISORDERLAUNCHER" title="Perfiles" description="Elige con qué nombre juegas. Solo uno puede estar en uso a la vez." />

  {#if error}<p class="form-error" role="alert">{error}</p>{/if}

  <section class="settings-panel panel" aria-label="Perfiles guardados">
    <div class="section-title">
      <h3>Tus perfiles</h3>
      <span>{accounts.list.length} {accounts.list.length === 1 ? 'perfil' : 'perfiles'}</span>
    </div>

    {#each accounts.list as account (account.id)}
      {@const status = stateText(accounts.states[account.id])}
      <div class="account-row" class:active={account.id === accounts.defaultId}>
        <Avatar />
        <div class="account-main">
          <h3>{account.label}</h3>
          <p class="subtle">
            {providerLabel[account.provider]}{account.id === accounts.defaultId ? ' · En uso' : ''}
          </p>
          {#if status}<p class="subtle" class:warn={needsSignIn(account.id)}>{status}</p>{/if}
        </div>
        <div class="account-actions">
          {#if account.provider === 'microsoft' && needsSignIn(account.id) && accounts.microsoft.enabled}
            <Button variant="primary" disabled={accounts.microsoft.loginInProgress} onclick={connect}>Volver a iniciar sesión</Button>
          {/if}
          {#if account.id === accounts.defaultId}
            <span class="pill"><span class="status-dot"></span>EN USO</span>
          {:else}
            <Button onclick={() => use(account.id)}>Usar</Button>
          {/if}
          <Button variant="ghost" onclick={() => (toRemove = account)}>Quitar</Button>
        </div>
      </div>
    {/each}

    <div class="account-row">
      <Icon name="grid" size={30} />
      <div class="account-main">
        <h3>Microsoft</h3>
        {#if !accounts.microsoft.enabled}
          <p class="subtle">El inicio de sesión con Microsoft no está activado en esta versión. Offline Gaming funciona con normalidad.</p>
          <details>
            <summary>Opciones avanzadas</summary>
            <p class="subtle">Si tienes una aplicación registrada en Azure, indica su identificador en Ajustes › Avanzado.</p>
          </details>
        {:else if accounts.microsoft.loginInProgress}
          <p class="subtle" role="status">Completa el inicio de sesión en tu navegador. Esta ventana se actualizará sola.</p>
        {:else}
          <p class="subtle">Conecta tu cuenta para jugar con tu perfil oficial. Se abrirá tu navegador.</p>
        {/if}
        {#if accounts.loginMessage}<p class="subtle" role="status"><strong>{accounts.loginMessage}</strong></p>{/if}
      </div>
      <div class="account-actions">
        {#if accounts.microsoft.loginInProgress}
          <Button onclick={() => accounts.cancelLogin()}>Cancelar</Button>
        {:else}
          <Button disabled={!accounts.microsoft.enabled} onclick={connect}>Conectar cuenta</Button>
        {/if}
      </div>
    </div>
  </section>

  <section class="settings-panel panel">
    <form
      class="form-stack"
      onsubmit={(e) => {
        e.preventDefault()
        add()
      }}
    >
      <div class="section-title"><h3>Añadir perfil sin conexión</h3></div>
      <p class="subtle">Offline Gaming: juega sin cuenta, con el nombre que elijas.</p>
      <Input label="Nombre" bind:value={name} {error} maxlength={16} placeholder="Alex" />
      <div>
        <Button type="submit" disabled={busy || name.trim().length === 0}>Crear perfil</Button>
      </div>
    </form>
  </section>

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
    gap: 20px;
    max-width: 800px;
  }

  .warn {
    color: var(--text-primary);
    font-weight: 600;
  }

  details {
    color: var(--text-secondary);
    font-size: 12px;
  }

  summary {
    cursor: pointer;
    min-height: 32px;
    display: flex;
    align-items: center;
  }
</style>
