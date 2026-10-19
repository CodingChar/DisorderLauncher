<script lang="ts">
  import { onMount } from 'svelte'
  import { pushBack } from '../lib/mc/back'
  import McButton from '../lib/mc/McButton.svelte'
  import McDialog from '../lib/mc/McDialog.svelte'
  import McList from '../lib/mc/McList.svelte'
  import McScreen from '../lib/mc/McScreen.svelte'
  import McText from '../lib/mc/McText.svelte'
  import { useGui } from '../lib/mc/gui'
  import type { RowSpec } from '../lib/mc/listTypes'
  import { bandWidth } from '../lib/mc/pos'
  import { findItemIcon } from '../lib/itemIcons'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { skins } from '../lib/stores/skins.svelte'
  import { providerLabel } from '../lib/text'
  import type { AccountRecord, AuthState } from '../lib/types'
  import ProfileForm from './parts/ProfileForm.svelte'

  const gui = useGui()
  const bw = $derived(bandWidth(gui.W))

  let sub = $state<'main' | 'new'>('main')
  let selectedId = $state<string | null>(accounts.defaultId)
  let error = $state<string | null>(null)
  let toRemove = $state<AccountRecord | null>(null)

  onMount(() => {
    accounts.loadMicrosoft()
    accounts.refreshStates()
    if (!skins.loaded) skins.load()
  })

  $effect(() => {
    if (sub === 'main') return
    return pushBack(() => (sub = 'main'))
  })

  const selected = $derived(accounts.list.find((a) => a.id === selectedId) ?? null)
  const showMicrosoft = $derived(accounts.microsoft.enabled || accounts.list.some((a) => a.provider === 'microsoft'))

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
      if (selectedId === target.id) selectedId = accounts.defaultId
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

  function row(a: AccountRecord): RowSpec {
    const skinId = a.providerData.type === 'local' ? a.providerData.skinId : null
    const face = skinId ? skins.previews[skinId] : undefined
    const grass = findItemIcon('grass-block')
    const status = stateText(accounts.states[a.id])
    const active = a.id === accounts.defaultId
    return {
      icon: face
        ? [
            { src: face, crop: [8, 8, 8, 8], imageWidth: 64 },
            { src: face, crop: [40, 8, 8, 8], imageWidth: 64 },
          ]
        : grass
          ? { src: grass.src }
          : undefined,
      lines: [
        { text: a.label },
        { text: providerLabel[a.provider] },
        active ? { text: 'Perfil activo', color: '#55ff55' } : { text: status ?? '', color: needsSignIn(a.id) ? '#ffaa00' : '#808080' },
      ],
    }
  }

  // Un solo mensaje bajo la lista: lo mas importante primero.
  const message = $derived.by((): { text: string; color: string; role?: 'alert' | 'status' } | null => {
    if (error) return { text: error, color: '#ff5555', role: 'alert' }
    if (accounts.microsoft.loginInProgress) return { text: 'Completa el inicio de sesión en tu navegador. Esta ventana se actualizará sola.', color: '#a0a0a0', role: 'status' }
    if (accounts.loginMessage) return { text: accounts.loginMessage, color: '#ffffff', role: 'status' }
    const status = selected ? stateText(accounts.states[selected.id]) : null
    if (status && selected && needsSignIn(selected.id)) return { text: `${selected.label}: ${status}.`, color: '#ffaa00' }
    if (showMicrosoft && !accounts.microsoft.enabled) return { text: 'El inicio de sesión con Microsoft no está activado en esta versión. Offline Gaming funciona con normalidad.', color: '#808080' }
    return null
  })
</script>

{#if sub === 'new'}
  <ProfileForm title="Añadir perfil" hint="Offline Gaming: juega sin cuenta, con el nombre que elijas." oncancel={() => (sub = 'main')} ondone={() => (sub = 'main')} />
{:else}
  <McScreen title="Perfiles">
    <McList
      items={accounts.list}
      getId={(a) => a.id}
      {row}
      bind:selectedId
      onactivate={(a) => use(a.id)}
      x={-(bw >> 1)}
      y={22}
      w={bw}
      h={gui.H - 118}
      empty="Aún no tienes perfiles. Crea uno con «Añadir»."
      ariaLabel="Perfiles guardados"
    />
    {#if message}<McText text={message.text} x={-155} y={gui.H - 92} w={310} align="center" color={message.color} role={message.role} maxLines={2} />{/if}

    <McButton anchor="bottom" x={-155} y={-52} w={100} label="Usar" ariaLabel="Usar este perfil" disabled={!selected || selected.id === accounts.defaultId} onclick={() => selected && use(selected.id)} />
    <McButton anchor="bottom" x={-50} y={-52} w={100} label="Quitar..." ariaLabel="Quitar este perfil" disabled={!selected} onclick={() => (toRemove = selected)} />
    <McButton anchor="bottom" x={55} y={-52} w={100} label="Añadir..." ariaLabel="Añadir perfil sin conexión" onclick={() => (sub = 'new')} />
    {#if showMicrosoft}
      {#if accounts.microsoft.loginInProgress}
        <McButton anchor="bottom" x={-50} y={-28} w={100} label="Cancelar" ariaLabel="Cancelar el inicio de sesión" onclick={() => accounts.cancelLogin()} />
      {:else if selected && selected.provider === 'microsoft' && needsSignIn(selected.id) && accounts.microsoft.enabled}
        <McButton anchor="bottom" x={-50} y={-28} w={100} label="Reconectar" ariaLabel="Volver a iniciar sesión" onclick={connect} />
      {:else}
        <McButton anchor="bottom" x={-50} y={-28} w={100} label="Conectar cuenta" ariaLabel="Conectar cuenta de Microsoft" disabled={!accounts.microsoft.enabled} onclick={connect} />
      {/if}
    {/if}
    <McButton anchor="bottom" x={55} y={-28} w={100} label="Atrás" onclick={() => nav.go('menu')} />

    {#if toRemove}
      <McDialog
        title="¿Quitar este perfil?"
        message={toRemove.provider === 'microsoft'
          ? `Se cerrará la sesión de «${toRemove.label}» en este equipo. Tus instancias no se borran.`
          : `Se quitará el perfil «${toRemove.label}». Tus instancias no se borran.`}
        buttons={[
          { label: 'Cancelar', onclick: () => (toRemove = null) },
          { label: 'Quitar', onclick: confirmRemove },
        ]}
        cancelIndex={0}
        onclose={() => (toRemove = null)}
      />
    {/if}
  </McScreen>
{/if}
