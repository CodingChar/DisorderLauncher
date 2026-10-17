<script lang="ts">
  import { onMount, tick } from 'svelte'
  import './app.css'
  import Button from './lib/components/Button.svelte'
  import ProfileBadge from './lib/components/ProfileBadge.svelte'
  import { accounts } from './lib/stores/accounts.svelte'
  import { instances } from './lib/stores/instances.svelte'
  import { nav, type View } from './lib/stores/nav.svelte'
  import { sessions } from './lib/stores/sessions.svelte'
  import AccountPicker from './routes/AccountPicker.svelte'
  import CreateInstance from './routes/CreateInstance.svelte'
  import EditInstance from './routes/EditInstance.svelte'
  import Home from './routes/Home.svelte'
  import Mods from './routes/Mods.svelte'
  import Settings from './routes/Settings.svelte'
  import Skins from './routes/Skins.svelte'
  import { settings } from './lib/stores/settings.svelte'
  import Welcome from './routes/Welcome.svelte'

  const items: { view: View; label: string }[] = [
    { view: 'home', label: 'Inicio' },
    { view: 'create', label: 'Nueva instancia' },
    { view: 'accounts', label: 'Perfiles' },
    { view: 'skins', label: 'Skins' },
    { view: 'settings', label: 'Ajustes' },
  ]

  const needsProfile = $derived(accounts.loaded && accounts.list.length === 0)

  // Al cambiar de pantalla el foco pasa al titulo: lectores de pantalla y teclado no se quedan
  // en un boton que ya no existe.
  let firstView = true
  $effect(() => {
    nav.view
    if (firstView) {
      firstView = false
      return
    }
    tick().then(() => {
      const heading = document.querySelector<HTMLElement>('main h1')
      heading?.setAttribute('tabindex', '-1')
      heading?.focus({ preventScroll: true })
    })
  })

  onMount(async () => {
    await sessions.init()
    await accounts.listen()
    await Promise.all([settings.load(), accounts.load(), instances.load(), accounts.loadMicrosoft()])
  })
</script>

{#if !accounts.loaded}
  <div class="boot" role="status">Cargando…</div>
{:else if accounts.loadError}
  <div class="boot" role="alert">
    <div class="boot-box">
      <p>{accounts.loadError}</p>
      <Button onclick={() => accounts.load()}>Reintentar</Button>
    </div>
  </div>
{:else if needsProfile}
  <Welcome />
{:else}
  <a class="skip" href="#main" onclick={(e) => { e.preventDefault(); document.getElementById('main')?.focus() }}>Saltar al contenido</a>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <small>LAUNCHER</small>
        <strong>Disorder</strong>
      </div>

      <nav class="nav" aria-label="Principal">
        {#each items as item (item.view)}
          <button class:active={nav.view === item.view} aria-current={nav.view === item.view ? 'page' : undefined} onclick={() => nav.go(item.view)}>
            {item.label}
          </button>
        {/each}
      </nav>

      <div class="spacer"></div>

      {#if accounts.current}
        <div class="profile-box">
          <ProfileBadge account={accounts.current} />
        </div>
      {/if}
    </aside>

    <main class="main" id="main" tabindex="-1">
      {#if nav.view === 'home'}
        <Home />
      {:else if nav.view === 'create'}
        <CreateInstance />
      {:else if nav.view === 'edit'}
        <EditInstance />
      {:else if nav.view === 'mods'}
        <Mods />
      {:else if nav.view === 'skins'}
        <Skins />
      {:else if nav.view === 'settings'}
        <Settings />
      {:else}
        <AccountPicker />
      {/if}
    </main>
  </div>
{/if}

<style>
  .boot {
    min-height: 100vh;
    display: grid;
    place-items: center;
    color: var(--text-secondary);
    padding: 24px;
    text-align: center;
  }

  .boot-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .boot-box p {
    margin: 0;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
  }

  .sidebar {
    background: var(--surface);
    border-right: 1px solid var(--border);
    padding: 20px 14px;
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: sticky;
    top: 0;
  }

  .brand {
    border: 1px solid var(--border-strong);
    border-radius: 0 var(--r-lg) var(--r-lg) 0;
    padding: 14px 12px;
    margin-bottom: 26px;
  }

  .brand small {
    display: block;
    color: var(--text-secondary);
    font-size: 10px;
    letter-spacing: 0.16em;
    margin-bottom: 4px;
  }

  .brand strong {
    font-family: var(--font-display);
    font-size: 18px;
    letter-spacing: -0.035em;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .nav button {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    padding: 12px;
    text-align: left;
    border-radius: var(--r-sm);
    min-height: 44px;
    transition: background 0.18s, color 0.18s;
  }

  .nav button:hover {
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  .nav button.active {
    background: var(--action);
    color: var(--action-text);
    border-radius: 0 var(--r-md) var(--r-md) 0;
    font-weight: 700;
  }

  .spacer {
    flex: 1;
  }

  .profile-box {
    border-top: 1px solid var(--border);
    padding-top: 14px;
  }

  .main {
    padding: 24px;
    min-width: 0;
    overflow-y: auto;
    height: 100vh;
  }

  @media (max-width: 720px) {
    .shell {
      grid-template-columns: 1fr;
    }

    .sidebar {
      height: auto;
      position: static;
      flex-direction: row;
      align-items: center;
      overflow-x: auto;
      gap: 12px;
    }

    .brand {
      margin: 0;
    }

    .nav {
      flex-direction: row;
    }

    .main {
      height: auto;
    }
  }
</style>
