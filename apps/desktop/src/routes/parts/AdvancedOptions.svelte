<script lang="ts">
  import McButton from '../../lib/mc/McButton.svelte'
  import McCycle from '../../lib/mc/McCycle.svelte'
  import McField from '../../lib/mc/McField.svelte'
  import McLabel from '../../lib/mc/McLabel.svelte'
  import McScreen from '../../lib/mc/McScreen.svelte'
  import McSlider from '../../lib/mc/McSlider.svelte'
  import McText from '../../lib/mc/McText.svelte'
  import { useGui } from '../../lib/mc/gui'
  import { JAVA_MODES, MEMORY_HINT, MEMORY_MODES, formatGb } from '../../lib/instanceForm'

  interface Props {
    memoryMode: 'auto' | 'fixed'
    maxGb: number
    javaMode: 'auto' | 'path'
    javaPath: string
    disabled?: boolean
    onback: () => void
    /** Si se da, aparece el acceso a los ajustes de Java (solo con una instancia ya creada). */
    onjava?: () => void
    /** Si se da, aparece «Recomendado para mi equipo»; devuelve el texto que explica lo que se aplico. */
    onrecommend?: () => Promise<string>
  }

  let { memoryMode = $bindable(), maxGb = $bindable(), javaMode = $bindable(), javaPath = $bindable(), disabled = false, onback, onjava, onrecommend }: Props = $props()

  const gui = useGui()
  let recommended = $state<string | null>(null)
  let recommending = $state(false)

  async function recommend() {
    if (!onrecommend || recommending) return
    recommending = true
    try {
      recommended = await onrecommend()
    } catch (e) {
      recommended = (e as Error).message ?? String(e)
    } finally {
      recommending = false
    }
  }

  const rowY = $derived(javaMode === 'path' ? 160 : 124)
</script>

<McScreen title="Opciones avanzadas">
  <McCycle label="Memoria" options={MEMORY_MODES} bind:value={memoryMode} y={26} {disabled} />
  {#if memoryMode === 'fixed'}
    <McSlider bind:value={maxGb} min={1} max={16} step={1} y={50} {disabled} format={(v) => `Memoria máxima: ${formatGb(v)}`} label="Memoria máxima en gigabytes" />
    <McText text={MEMORY_HINT} x={-100} y={74} w={200} color="#808080" maxLines={2} />
  {/if}

  <McCycle label="Java" options={JAVA_MODES} bind:value={javaMode} y={98} {disabled} />
  {#if javaMode === 'path'}
    <McLabel x={-100} y={124} color="#a0a0a0" text="Carpeta de Java" />
    <McField bind:value={javaPath} y={134} {disabled} maxlength={260} placeholder={'C:\\Program Files\\Java\\jdk-21'} label="Carpeta de Java" hint={'La carpeta que contiene «bin».'} />
  {/if}

  {#if onjava}
    <McButton x={-100} y={rowY} label="Ajustes de Java..." ariaLabel="Ajustes de rendimiento de Java" {disabled} onclick={onjava} />
  {/if}
  {#if onrecommend}
    <McButton x={-100} y={rowY + (onjava ? 24 : 0)} label={recommending ? 'Mirando tu equipo...' : 'Recomendado para mi equipo'} ariaLabel="Aplicar el rendimiento recomendado para este equipo" disabled={disabled || recommending} onclick={() => void recommend()} />
    {#if recommended}
      <McText text={recommended} x={-155} y={rowY + (onjava ? 48 : 24)} w={310} color="#55ff55" maxLines={3} role="status" />
    {/if}
  {/if}

  {#if !recommended}
    <McText text="Puedes dejarlas como están: no hace falta tocar nada para jugar." x={-155} y={gui.H - 50} w={310} align="center" color="#707070" />
  {/if}
  <McButton anchor="bottom" y={-28} label="Hecho" onclick={onback} />
</McScreen>
