<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from './lib/api'
  import { goBack, runBack, trackView } from './lib/mc/back'
  import McButton from './lib/mc/McButton.svelte'
  import McDialog from './lib/mc/McDialog.svelte'
  import McLabel from './lib/mc/McLabel.svelte'
  import McScreen from './lib/mc/McScreen.svelte'
  import McStage from './lib/mc/McStage.svelte'
  import McText from './lib/mc/McText.svelte'
  import { accounts } from './lib/stores/accounts.svelte'
  import { instances } from './lib/stores/instances.svelte'
  import { nav } from './lib/stores/nav.svelte'
  import { sessions } from './lib/stores/sessions.svelte'
  import { settings } from './lib/stores/settings.svelte'
  import { HELP_TEXT } from './lib/helpText'
  import AccountPicker from './routes/AccountPicker.svelte'
  import CreateInstance from './routes/CreateInstance.svelte'
  import EditInstance from './routes/EditInstance.svelte'
  import Explore from './routes/Explore.svelte'
  import Home from './routes/Home.svelte'
  import Instances from './routes/Instances.svelte'
  import MainMenu from './routes/MainMenu.svelte'
  import Mods from './routes/Mods.svelte'
  import Settings from './routes/Settings.svelte'
  import SkinEditor from './routes/SkinEditor.svelte'
  import Skins from './routes/Skins.svelte'
  import Tools from './routes/Tools.svelte'
  import Welcome from './routes/Welcome.svelte'

  let notices = $state<string[]>([])
  let helpOpen = $state(false)
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine)
  let themeError = $state<string | null>(null)

  const needsProfile = $derived(accounts.loaded && !accounts.loadError && accounts.list.length === 0)
  const showPanorama = $derived(accounts.loaded && !accounts.loadError && (needsProfile || nav.view === 'menu'))

  // Vista anterior: edicion y mods vuelven a donde se abrieron.
  $effect(() => {
    trackView(nav.view)
  })

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.ctrlKey || event.metaKey || event.altKey || event.defaultPrevented) return
    if (needsProfile || !accounts.loaded) return
    event.preventDefault()
    // Esc en un campo solo sale del campo; la siguiente pulsacion es «Atras».
    if (event.target instanceof HTMLInputElement) {
      event.target.blur()
      return
    }
    if (!runBack() && nav.view !== 'menu') goBack()
  }

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

<svelte:window onkeydown={onKeydown} />

<McStage panorama={showPanorama}>
  {#if !accounts.loaded}
    <McScreen title="DisorderLauncher">
      <McLabel center y={110} color="#a0a0a0" text="Cargando..." />
    </McScreen>
  {:else if accounts.loadError}
    <McScreen title="DisorderLauncher">
      <McText text={accounts.loadError} x={-155} y={90} w={310} align="center" color="#ff5555" maxLines={5} role="alert" />
      <McButton label="Reintentar" anchor="bottom" y={-28} onclick={() => accounts.load()} />
    </McScreen>
  {:else if needsProfile}
    <Welcome />
  {:else if nav.view === 'menu'}
    <MainMenu {online} {themeError} onhelp={() => (helpOpen = true)} />
  {:else if nav.view === 'home'}
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

  {#if helpOpen}
    <McDialog title="Ayuda y soporte" message={HELP_TEXT} scroll buttons={[{ label: 'Hecho', onclick: () => (helpOpen = false) }]} onclose={() => (helpOpen = false)} />
  {/if}
  {#if notices.length > 0 && !helpOpen}
    <McDialog title="Aviso" message={notices.join('\n')} scroll buttons={[{ label: 'Entendido', onclick: () => (notices = []) }]} onclose={() => (notices = [])} />
  {/if}
</McStage>
