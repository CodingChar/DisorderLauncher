<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Card from '../lib/components/Card.svelte'
  import Input from '../lib/components/Input.svelte'
  import ProfileBadge from '../lib/components/ProfileBadge.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'

  let name = $state('')
  let error = $state<string | null>(null)
  let busy = $state(false)

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
</script>

<div class="page">
  <header>
    <small>PERFILES</small>
    <h1>¿Quién juega?</h1>
  </header>

  <ul class="list">
    {#each accounts.list as account (account.id)}
      <li>
        <Card>
          <div class="row">
            <ProfileBadge {account} />
            {#if account.id === accounts.defaultId}
              <span class="current">En uso</span>
            {:else}
              <Button onclick={() => use(account.id)}>Usar</Button>
            {/if}
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
      <h3>Añadir otro perfil</h3>
      <Input label="Nombre" bind:value={name} {error} maxlength={16} placeholder="Alex" />
      <div>
        <Button type="submit" disabled={busy || name.trim().length === 0}>Crear perfil</Button>
      </div>
    </form>
  </Card>

  <Card>
    <div class="row">
      <div>
        <h3>Microsoft</h3>
        <p class="soon">Podrás conectar tu cuenta de Microsoft más adelante.</p>
      </div>
      <Button disabled>Conectar cuenta</Button>
    </div>
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

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
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
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
</style>
