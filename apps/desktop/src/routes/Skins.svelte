<script lang="ts">
  import { onMount } from 'svelte'
  import Button from '../lib/components/Button.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import Card from '../lib/components/Card.svelte'
  import ConfirmDialog from '../lib/components/ConfirmDialog.svelte'
  import Input from '../lib/components/Input.svelte'
  import Select from '../lib/components/Select.svelte'
  import SkinPreview from '../lib/components/SkinPreview.svelte'
  import { accounts } from '../lib/stores/accounts.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { skins } from '../lib/stores/skins.svelte'
  import type { SkinAsset } from '../lib/types'

  const localProfiles = $derived(accounts.list.filter((a) => a.provider === 'local'))

  let profileId = $state('')
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)
  let busy = $state(false)
  let toRemove = $state<SkinAsset | null>(null)
  let renaming = $state<SkinAsset | null>(null)
  let newName = $state('')

  onMount(() => {
    skins.load()
  })

  // El perfil elegido arranca en el predeterminado si es local, o en el primero local.
  $effect(() => {
    if (profileId && localProfiles.some((a) => a.id === profileId)) return
    const preferred = localProfiles.find((a) => a.id === accounts.defaultId) ?? localProfiles[0]
    profileId = preferred?.id ?? ''
  })

  const profile = $derived(localProfiles.find((a) => a.id === profileId))
  const profileSkinId = $derived(profile?.providerData.type === 'local' ? profile.providerData.skinId : null)

  async function run(action: () => Promise<string | void>) {
    if (busy) return
    busy = true
    error = null
    notice = null
    try {
      const message = await action()
      if (message) notice = message
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }

  const openEditor = (id: string | null) => {
    skins.editingId = id
    nav.go('skinEditor')
  }

  const importSkin = () =>
    run(async () => {
      const skin = await skins.importSkin()
      return skin ? `Se agregó «${skin.name}».` : undefined
    })

  const use = (skin: SkinAsset) =>
    run(async () => {
      await skins.assign(profileId, skin.id)
      return `«${skin.name}» es ahora la skin de ${profile?.label}.`
    })

  const unassign = () =>
    run(async () => {
      await skins.assign(profileId, null)
      return `${profile?.label} ya no tiene skin asignada.`
    })

  const exportSkin = (skin: SkinAsset) =>
    run(async () => ((await skins.exportSkin(skin.id)) ? 'Skin guardada.' : undefined))

  const confirmRemove = () => {
    const target = toRemove
    toRemove = null
    if (target) run(async () => (await skins.remove(target.id), 'Skin eliminada.'))
  }

  const startRename = (skin: SkinAsset) => {
    renaming = skin
    newName = skin.name
  }

  const saveRename = () => {
    const target = renaming
    renaming = null
    if (target) run(() => skins.rename(target.id, newName))
  }
</script>

<div class="page">
  <PageHeading eyebrow="DISORDERLAUNCHER" title="Skins" description="Tu biblioteca de skins: impórtalas, edítalas y asígnalas a un perfil sin conexión." />

  <Card>
    <div class="toolbar">
      <div class="profile">
        {#if localProfiles.length > 0}
          <Select
            label="Perfil sin conexión"
            bind:value={profileId}
            options={localProfiles.map((a) => ({ value: a.id, label: a.label }))}
          />
        {:else}
          <p class="muted">Crea un perfil sin conexión para asignarle una skin.</p>
        {/if}
      </div>
      <div class="buttons">
        <Button variant="primary" disabled={busy} onclick={importSkin}>Importar skin…</Button>
        <Button disabled={busy} onclick={() => openEditor(null)}>Crear skin nueva</Button>
        <Button variant="ghost" disabled={busy || !profileSkinId} onclick={unassign}>Quitar skin del perfil</Button>
      </div>
    </div>
    <p class="muted fineprint">
      La skin se guarda aquí y se aplica dentro del juego al abrir una instancia de este perfil (Minecraft decide si los brazos son anchos o finos). En versiones anteriores a 1.8 la skin se adapta sola: sin segunda capa y con los brazos y piernas iguales. No se registra en tu cuenta de Microsoft.
    </p>
  </Card>

  {#if error}<p class="form-error" role="alert">{error}</p>{/if}
  {#if notice}<p class="form-ok" role="status">{notice}</p>{/if}

  {#if skins.loaded && skins.list.length === 0}
    <Card>
      <h3>Aún no tienes skins</h3>
      <p class="muted">Importa un archivo PNG de 64×64 (o 64×32) o crea una desde cero con el editor.</p>
    </Card>
  {:else}
    <ul class="grid">
      {#each skins.list as skin (skin.id)}
        <li>
          <Card>
            <div class="skin">
              {#if skins.previews[skin.id]}
                <SkinPreview src={skins.previews[skin.id]} model={skin.model} label={`Vista previa de ${skin.name}`} />
              {/if}
              <strong>{skin.name}</strong>
              <small>{skin.model === 'slim' ? 'Brazos finos (Alex)' : 'Brazos clásicos (Steve)'}</small>
              <div class="actions">
                {#if profileSkinId === skin.id}
                  <span class="current">En uso</span>
                {:else}
                  <Button disabled={busy || !profile} onclick={() => use(skin)}>Usar</Button>
                {/if}
                <Button disabled={busy} onclick={() => openEditor(skin.id)}>Editar</Button>
                <Button variant="ghost" disabled={busy} onclick={() => startRename(skin)}>Renombrar</Button>
                <Button
                  variant="ghost"
                  disabled={busy}
                  onclick={() => run(() => skins.setModel(skin.id, skin.model === 'slim' ? 'classic' : 'slim'))}
                >
                  Cambiar modelo
                </Button>
                <Button variant="ghost" disabled={busy} onclick={() => exportSkin(skin)}>Exportar</Button>
                <Button variant="ghost" disabled={busy} onclick={() => (toRemove = skin)}>Eliminar</Button>
              </div>
            </div>
          </Card>
        </li>
      {/each}
    </ul>
  {/if}

  {#if renaming}
    <Card>
      <form
        onsubmit={(e) => {
          e.preventDefault()
          saveRename()
        }}
      >
        <Input label="Nuevo nombre" bind:value={newName} maxlength={48} autofocus />
        <div class="buttons">
          <Button variant="primary" type="submit" disabled={newName.trim().length === 0}>Guardar nombre</Button>
          <Button variant="ghost" onclick={() => (renaming = null)}>Cancelar</Button>
        </div>
      </form>
    </Card>
  {/if}

  <ConfirmDialog
    open={toRemove !== null}
    title="¿Eliminar esta skin?"
    message={toRemove ? `Se borrará «${toRemove.name}» de tu biblioteca y se quitará de los perfiles que la usan.` : ''}
    confirmLabel="Eliminar"
    onconfirm={confirmRemove}
    oncancel={() => (toRemove = null)}
  />
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 1100px;
  }



  h3 {
    font-size: 18px;
    margin-bottom: 6px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: flex-end;
    justify-content: space-between;
  }

  .profile {
    min-width: 220px;
  }

  .buttons,
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .muted {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .fineprint {
    margin-top: 12px;
    font-size: 12px;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 14px;
  }

  .skin {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
    --w: 96px;
  }

  .skin small {
    color: var(--text-secondary);
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


</style>
