<script lang="ts">
  import { onMount, tick } from 'svelte'
  import './app.css'
  import Avatar from './lib/components/Avatar.svelte'
  import Button from './lib/components/Button.svelte'
  import HelpDialog from './lib/components/HelpDialog.svelte'
  import Icon from './lib/components/Icon.svelte'
  import { api } from './lib/api'
  import { accounts } from './lib/stores/accounts.svelte'
  import { instances } from './lib/stores/instances.svelte'
  import { nav, navParent, viewLabel, type View } from './lib/stores/nav.svelte'
  import { sessions } from './lib/stores/sessions.svelte'
  import { settings } from './lib/stores/settings.svelte'
  import { APP_VERSION } from './lib/version'
  import AccountPicker from './routes/AccountPicker.svelte'
  import CreateInstance from './routes/CreateInstance.svelte'
  import EditInstance from './routes/EditInstance.svelte'
  import Explore from './routes/Explore.svelte'
  import Home from './routes/Home.svelte'
  import Instances from './routes/Instances.svelte'
  import Mods from './routes/Mods.svelte'
  import Settings from './routes/Settings.svelte'
  import SkinEditor from './routes/SkinEditor.svelte'
  import Skins from './routes/Skins.svelte'
  import Tools from './routes/Tools.svelte'
  import Welcome from './routes/Welcome.svelte'

  const items: { view: View; label: string; icon: string }[] = [
    { view: 'home', label: 'Inicio', icon: 'home' },
    { view: 'instances', label: 'Mis instancias', icon: 'box' },
    { view: 'explore', label: 'Explorar', icon: 'compass' },
    { view: 'tools', label: 'Herramientas', icon: 'wrench' },
    { view: 'accounts', label: 'Perfiles', icon: 'user' },
    { view: 'skins', label: 'Skins', icon: 'shirt' },
  ]

  let notices = $state<string[]>([])
  let helpOpen = $state(false)
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine)
  let themeError = $state<string | null>(null)

  const needsProfile = $derived(accounts.loaded && accounts.list.length === 0)
  const activeNav = $derived(navParent[nav.view])

  async function toggleTheme() {
    themeError = null
    try {
      await settings.toggleBlue()
    } catch (e) {
      themeError = (e as Error).message
    }
  }

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
      window.scrollTo({ top: 0 })
    })
  })

  onMount(() => {
    const goOnline = () => (online = true)
    const goOffline = () => (online = false)
    window.addEventListener('online', goOnline)
    window.addEventListener('offline', goOffline)

    ;(async () => {
      // Suscripciones y lecturas iniciales a la vez: ninguna depende de otra y el arranque tarda lo de la mas lenta.
      await Promise.all([sessions.init(), accounts.listen(), settings.load(), accounts.load(), instances.load(), accounts.loadMicrosoft()])
      notices = await api.startupNotices().catch(() => [])
    })()

    return () => {
      window.removeEventListener('online', goOnline)
      window.removeEventListener('offline', goOffline)
    }
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
  <a
    class="skip"
    href="#main"
    onclick={(e) => {
      e.preventDefault()
      document.getElementById('main')?.focus()
    }}>Saltar al contenido</a
  >
  <div class="app">
    <aside class="sidebar">
      <button type="button" class="brand" aria-label="DisorderLauncher: ir a Inicio" onclick={() => nav.go('home')}>
        <div>disorder<span>LAUNCHER</span></div>
      </button>

      <div class="workspace-label">TU ESPACIO DE JUEGO</div>

      <nav aria-label="Principal">
        {#each items as item (item.view)}
          <button
            class="nav-item"
            class:active={activeNav === item.view}
            aria-current={activeNav === item.view ? 'page' : undefined}
            aria-label={item.label}
            onclick={() => nav.go(item.view)}
          >
            <Icon name={item.icon} />
            {item.label}
            {#if item.view === 'instances' && instances.list.length > 0}
              <small>{String(instances.list.length).padStart(2, '0')}</small>
            {/if}
          </button>
        {/each}
      </nav>

      <div class="sidebar-bottom">
        <div class="offline-note" role="status">
          <span class="status-dot" class:warn={!online}></span>
          <span>
            {online ? 'Conectado' : 'Modo sin conexión'}
            <small>{online ? 'Descargas disponibles' : 'Puedes jugar lo ya descargado'}</small>
          </span>
        </div>

        <button
          class="nav-item"
          class:active={activeNav === 'settings'}
          aria-current={activeNav === 'settings' ? 'page' : undefined}
          aria-label="Ajustes"
          onclick={() => nav.go('settings')}
        >
          <Icon name="settings" />
          Ajustes
        </button>

        <button class="nav-item" aria-label="Ayuda y soporte" aria-haspopup="dialog" onclick={() => (helpOpen = true)}>
          <Icon name="help" />
          Ayuda y soporte
          <Icon name="arrow" size={16} />
        </button>

        {#if accounts.current}
          <button class="sidebar-profile" aria-label="Perfil activo: {accounts.current.label}. Cambiar de perfil" onclick={() => nav.go('accounts')}>
            <Avatar />
            <span>
              {accounts.current.label}
              <small>Perfil activo</small>
            </span>
            <Icon name="chevron" size={16} />
          </button>
        {/if}

        <div class="build-version">
          <span>DisorderLauncher</span>
          <span>v{APP_VERSION} <b>ALPHA</b></span>
        </div>
      </div>
    </aside>

    <main id="main" tabindex="-1">
      <div class="top-bar">
        <div class="breadcrumb">
          Launcher <span>/</span> <strong>{viewLabel[nav.view]}</strong>
        </div>
        <div class="header-actions">
          <span class="connection" role="status">
            <span class="status-dot" class:warn={!online}></span>
            <span class="connection-label">{online ? 'Todo listo para jugar' : 'Sin conexión: puedes jugar lo ya descargado'}</span>
          </span>
          <span class="header-divider"></span>
          <button
            class="icon-button"
            onclick={toggleTheme}
            aria-pressed={settings.isBlue}
            aria-label={settings.isBlue ? 'Quitar el tema azul oscuro' : 'Usar el tema azul oscuro'}
            title={settings.isBlue ? 'Quitar el tema azul oscuro' : 'Usar el tema azul oscuro'}
          >
            <Icon name="moon" size={19} />
          </button>
          {#if accounts.current}
            <button class="mini-profile" onclick={() => nav.go('accounts')} aria-label="Abrir perfiles">
              <Avatar />
            </button>
          {/if}
        </div>
      </div>

      <div class="content">
        {#if notices.length > 0}
          <div class="notice-bar" role="status">
            <div>{#each notices as n (n)}<p>{n}</p>{/each}</div>
            <Button onclick={() => (notices = [])}>Entendido</Button>
          </div>
        {/if}
        {#if themeError}
          <p class="form-error" role="alert">{themeError}</p>
        {/if}

        {#if nav.view === 'home'}
          <Home />
        {:else if nav.view === 'instances'}
          <Instances />
        {:else if nav.view === 'explore'}
          <Explore />
        {:else if nav.view === 'tools'}
          <Tools />
        {:else if nav.view === 'create'}
          <CreateInstance />
        {:else if nav.view === 'edit'}
          <EditInstance />
        {:else if nav.view === 'mods'}
          <Mods />
        {:else if nav.view === 'skins'}
          <Skins />
        {:else if nav.view === 'skinEditor'}
          <SkinEditor />
        {:else if nav.view === 'settings'}
          <Settings />
        {:else}
          <AccountPicker />
        {/if}

        <footer class="page-footer">
          <span>
            <span class="status-dot"></span>
            Puedes jugar sin conexión
          </span>
          <span>
            DisorderLauncher
            <span class="footer-mark" aria-hidden="true">d.</span>
          </span>
        </footer>
      </div>
    </main>
  </div>

  <HelpDialog open={helpOpen} onclose={() => (helpOpen = false)} />
{/if}
