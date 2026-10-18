<script lang="ts">
  import type { Optimization } from '../types'

  interface Props {
    value: Optimization
    disabled?: boolean
  }

  let { value = $bindable(), disabled = false }: Props = $props()

  const levels: { id: Optimization; label: string; hint: string }[] = [
    { id: 'off', label: 'Normal', hint: 'Sin cambios. Para equipos con buen rendimiento.' },
    { id: 'balanced', label: 'Equilibrado', hint: 'Menos tirones: distancia de visión moderada y menos partículas.' },
    { id: 'lowEnd', label: 'Equipo modesto', hint: 'Lo mínimo en gráficos y menos carga para el procesador y la memoria.' },
  ]
</script>

<fieldset class="group" {disabled}>
  <legend>Rendimiento</legend>
  {#each levels as level (level.id)}
    <label class="check-row">
      <input type="radio" bind:group={value} value={level.id} />
      <span>{level.label}<small>{level.hint}</small></span>
    </label>
  {/each}
  <p class="subtle">
    Cambia los ajustes gráficos iniciales (solo la primera vez, luego puedes cambiarlos en el juego) y la forma en que Java usa la memoria. En instancias con mods puedes añadir además mods de rendimiento desde «Mis mods».
  </p>
</fieldset>
