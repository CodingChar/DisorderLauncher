<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from './lib/api'
  import { playSfx } from './lib/audio/sfx'
  import { goBack, runBack, trackView } from './lib/mc/back'
  import McButton from './lib/mc/McButton.svelte'
  import McDetailHost from './lib/mc/McDetailHost.svelte'
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
  import { loadSkinEditor, prepareScreen, warmNavigation } from './lib/navigationWarmup'
  import AccountPicker from './routes/AccountPicker.svelte'
  import CreateInstance from './routes/CreateInstance.svelte'
  import EditInstance from './routes/EditInstance.svelte'
  import Explore from './routes/Explore.svelte'
  import Home from './routes/Home.svelte'
  import Instances from './routes/Instances.svelte'
  import MainMenu from './routes/MainMenu.svelte'
  import Mods from './routes/Mods.svelte'
  import Settings from './routes/Settings.svelte'
  import Skins from './routes/Skins.svelte'
  import Tools from './routes/Tools.svelte'
  import Welcome from './routes/Welcome.svelte'

  let notices = $state<string[]>([])
  let helpOpen = $state(false)
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine)
  let tourRequested = $state(false)

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
    if (runBack()) playSfx('back')
    else if (nav.view !== 'menu') {
      playSfx('back')
      goBack()
    }
  }

  onMount(() => {
    let alive = true
    let cancelWarmup = () => {}
    const stopPreparation = nav.setPreparation(prepareScreen)
    const goOnline = () => (online = true)
    const goOffline = () => (online = false)
    window.addEventListener('online', goOnline)
    window.addEventListener('offline', goOffline)
    // Si se borra o cambia una instancia desde el explorador, al volver al launcher la lista se pone al dia.
    const refreshOnFocus = () => {
      if (instances.loaded && instances.importPercent === null) void instances.load()
    }
    window.addEventListener('focus', refreshOnFocus)

    ;(async () => {
      // Suscripciones y lecturas iniciales a la vez: ninguna depende de otra y el arranque tarda lo de la mas lenta.
      await Promise.all([sessions.init(), accounts.listen(), settings.load(), accounts.load(), instances.load(), accounts.loadMicrosoft()])
      if (alive) cancelWarmup = warmNavigation()
      notices = await api.startupNotices().catch(() => [])
      // La revision de arranque puede haber limpiado o deshecho instancias: se vuelve a leer la lista.
      if (alive && notices.length > 0) await instances.load()
    })()

    return () => {
      alive = false
      cancelWarmup()
      stopPreparation()
      window.removeEventListener('online', goOnline)
      window.removeEventListener('offline', goOffline)
      window.removeEventListener('focus', refreshOnFocus)
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
    <MainMenu {online} {tourRequested} tourAllowed={!helpOpen && notices.length === 0} ontourend={() => (tourRequested = false)} onhelp={() => (helpOpen = true)} />
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
    {#await loadSkinEditor()}
      <McScreen title="Editor de skins"><McLabel center y={110} color="#a0a0a0" text="Cargando editor..." /></McScreen>
    {:then { default: SkinEditor }}
      <SkinEditor />
    {:catch}
      <McScreen title="Editor de skins">
        <McText text="No se pudo abrir el editor. Vuelve a intentarlo." x={-150} y={90} w={300} align="center" color="#ff5555" role="alert" />
        <McButton label="Volver" anchor="bottom" y={-28} onclick={() => nav.go('skins')} />
      </McScreen>
    {/await}
  {:else if nav.view === 'settings'}
    <Settings />
  {:else}
    <AccountPicker />
  {/if}

  {#if helpOpen}
    <McDialog title="Ayuda y soporte" message={HELP_TEXT} scroll buttons={[{ label: 'Ver tutorial', onclick: () => { helpOpen = false; nav.go('menu'); tourRequested = true } }, { label: 'Hecho', onclick: () => (helpOpen = false) }]} onclose={() => (helpOpen = false)} />
  {/if}
  {#if notices.length > 0 && !helpOpen}
    <McDialog title="Aviso" message={notices.join('\n')} scroll buttons={[{ label: 'Entendido', onclick: () => (notices = []) }]} onclose={() => (notices = [])} />
  {/if}
  <McDetailHost />
</McStage>
