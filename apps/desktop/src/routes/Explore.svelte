<script lang="ts">
  import Button from '../lib/components/Button.svelte'
  import Icon from '../lib/components/Icon.svelte'
  import PageHeading from '../lib/components/PageHeading.svelte'
  import ProgressBar from '../lib/components/ProgressBar.svelte'
  import { instances } from '../lib/stores/instances.svelte'
  import { nav } from '../lib/stores/nav.svelte'

  let importing = $state(false)
  let error = $state<string | null>(null)

  async function importPack() {
    if (importing) return
    importing = true
    error = null
    try {
      const instance = await instances.importModpack()
      if (instance) nav.go('home')
    } catch (e) {
      error = (e as Error).message
    } finally {
      importing = false
    }
  }
</script>

<PageHeading eyebrow="DISORDERLAUNCHER" title="Explorar" description="Tres formas de empezar una nueva aventura." />

<div class="explore-grid">
  <section class="explore-card panel">
    <div class="explore-art"><div class="block grass" aria-hidden="true"><span></span><i></i><b></b></div></div>
    <div class="explore-copy">
      <span class="eyebrow">SIN MODS</span>
      <h2>La esencia de Minecraft</h2>
      <p>Minecraft original, tal y como es. Elige la versión que quieras y empieza a jugar.</p>
      <Button onclick={() => nav.create('vanilla')}>Crear instancia original <Icon name="arrow" size={16} /></Button>
    </div>
  </section>

  <section class="explore-card panel">
    <div class="explore-art art-1"><div class="block stone" aria-hidden="true"><span></span><i></i><b></b></div></div>
    <div class="explore-copy">
      <span class="eyebrow">CON MODS</span>
      <h2>Un mundo a tu medida</h2>
      <p>Instancias con Fabric (también Quilt, Forge y NeoForge) para añadir mods: buscas en Modrinth y se instalan con lo que necesitan.</p>
      <Button onclick={() => nav.create('fabric')}>Crear instancia con mods <Icon name="arrow" size={16} /></Button>
    </div>
  </section>

  <section class="explore-card panel">
    <div class="explore-art art-2"><div class="block sand" aria-hidden="true"><span></span><i></i><b></b></div></div>
    <div class="explore-copy">
      <span class="eyebrow">MODPACK</span>
      <h2>Importa un modpack</h2>
      <p>Si tienes un archivo <strong>.mrpack</strong> (Modrinth), se crea la instancia con todo lo necesario.</p>
      {#if importing && instances.importPercent !== null}
        <ProgressBar percent={instances.importPercent} label="Importando modpack" />
      {/if}
      <Button disabled={importing} onclick={importPack}>{importing ? 'Importando…' : 'Importar modpack…'}</Button>
      {#if error}<small class="form-error" role="alert">{error}</small>{/if}
    </div>
  </section>
</div>
