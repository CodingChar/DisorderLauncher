<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Icon from '../lib/components/Icon.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import { api } from '../lib/api'
  import { FEATURES } from '../lib/features'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'
  import { sessions } from '../lib/stores/sessions.svelte'

  const instance = $derived(instances.selected)
  const busy = $derived(instance ? sessions.isBusy(instance.id) : false)

  let working = $state(false)
  let error = $state<string | null>(null)
  let notice = $state<string | null>(null)

  async function run(action: () => Promise<string | void>) {
    if (working) return
    working = true
    error = null
    notice = null
    try {
      const message = await action()
      if (message) notice = message
    } catch (e) {
      error = (e as Error).message
    } finally {
      working = false
    }
  }

  function repair() {
    if (!instance) return
    sessions.repair(instance.id)
    nav.go('home')
  }

  const exportPack = () =>
    run(async () => {
      if (!instance) return
      const summary = await api.modpacksExport(instance.id)
      return summary ? `Modpack guardado: ${summary.remoteFiles} mods por enlace y ${summary.bundledFiles} archivos incluidos.` : undefined
    })

  const importPack = () =>
    run(async () => {
      const created = await instances.importModpack()
      if (created) nav.go('home')
    })

  /** Herramientas que todavia no existen: no se muestran (ver `FEATURES`) y, si se activan, nunca fingen funcionar. */
  const soon = [
    { icon: 'archive', name: 'Copia de seguridad', description: 'Guarda tus instancias y su configuración en un archivo comprimido.' },
    { icon: 'clipboard', name: 'Informe de diagnóstico', description: 'Resume tu equipo y las últimas sesiones para pedir ayuda.' },
    { icon: 'trash', name: 'Limpiar caché', description: 'Borra descargas y archivos temporales que ya no usas.' },
  ]
</script>

<PageHeading eyebrow="DISORDERLAUNCHER" title="Herramientas" description="Utilidades para cuidar tus instancias." />

<div class="library-toolbar">
  <span>{instance ? `Instancia activa: ${instance.name}` : 'No hay ninguna instancia activa'}</span>
  <span class="tiny-label">LAS ACCIONES USAN LA INSTANCIA ACTIVA</span>
</div>

{#if error}<p class="form-error" role="alert">{error}</p>{/if}
{#if notice}<p class="form-ok" role="status">{notice}</p>{/if}

<div class="tool-grid">
  <section class="tool-card panel">
    <div class="tool-icon"><Icon name="zap" size={22} /></div>
    <h3>Reparar instancia</h3>
    <p>Comprueba los archivos de la instancia activa y vuelve a descargar lo que falte o esté dañado.</p>
    <div class="tool-foot">
      <Button disabled={!instance || busy} onclick={repair}>Reparar</Button>
    </div>
  </section>

  <section class="tool-card panel">
    <div class="tool-icon"><Icon name="download" size={22} /></div>
    <h3>Exportar modpack</h3>
    <p>Guarda la instancia activa como un archivo .mrpack para compartirla. No incluye mundos ni registros.</p>
    <div class="tool-foot">
      <Button disabled={!instance || working} onclick={exportPack}>Exportar</Button>
    </div>
  </section>

  <section class="tool-card panel">
    <div class="tool-icon"><Icon name="grid" size={22} /></div>
    <h3>Importar modpack</h3>
    <p>Crea una instancia nueva a partir de un archivo .mrpack de Modrinth.</p>
    {#if working && instances.importPercent !== null}
      <ProgressBar percent={instances.importPercent} label="Importando modpack" />
    {/if}
    <div class="tool-foot">
      <Button disabled={working} onclick={importPack}>Importar</Button>
    </div>
  </section>

  {#each FEATURES.comingSoonTools ? soon : [] as tool (tool.name)}
    <section class="tool-card panel">
      <div class="tool-icon"><Icon name={tool.icon} size={22} /></div>
      <h3>{tool.name}</h3>
      <p>{tool.description}</p>
      <div class="tool-foot">
        <span class="tiny-label">PRÓXIMAMENTE</span>
        <Button disabled>No disponible</Button>
      </div>
    </section>
  {/each}
</div>
