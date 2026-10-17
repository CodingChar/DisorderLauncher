<script lang="ts">
  import { untrack } from 'svelte'
  import {
    compatibilityModes,
    iconItems,
    javaRuntimes,
    loaders,
    mcVersions,
    type Instance,
  } from './types'
  import ItemLogo from './ItemLogo.svelte'

  let {
    initial,
    editing,
    onSave,
    onCancel,
  }: {
    initial: Instance
    editing: boolean
    onSave: (v: Instance) => void
    onCancel: () => void
  } = $props()

  // `initial` solo se lee al montar: el borrador evoluciona de forma independiente y
  // `App` remonta el componente con `{#key}` cuando cambia de instancia.
  let draft = $state<Instance>(untrack(() => ({ ...initial })))
  let error = $state('')
  let advancedRef = $state<HTMLDetailsElement>()
  let iconDialogRef = $state<HTMLDialogElement>()

  function update<K extends keyof Instance>(key: K, value: Instance[K]) {
    draft = { ...draft, [key]: value }
    // El aviso anterior deja de ser válido en cuanto el usuario toca un campo.
    error = ''
  }

  function openAdvanced() {
    if (advancedRef) advancedRef.open = true
  }

  /**
   * Los campos de "Opciones avanzadas" viven dentro de un `<details>` cerrado, así
   * que el navegador los marca inválidos pero no puede mostrar el aviso. Abrimos el
   * desplegable en fase de captura para que el error quede visible.
   */
  function revealInvalidFields(node: HTMLFormElement) {
    const handler = (event: Event) => {
      const field = event.target as HTMLElement
      if (advancedRef?.contains(field)) openAdvanced()
    }
    node.addEventListener('invalid', handler, true)
    return { destroy: () => node.removeEventListener('invalid', handler, true) }
  }

  /** Sustituye al atributo `autofocus`, que Svelte marca como mala práctica. */
  function autofocus(node: HTMLElement) {
    node.focus()
  }

  function openIconPicker() {
    iconDialogRef?.showModal()
    iconDialogRef
      ?.querySelector<HTMLButtonElement>('[aria-pressed="true"]')
      ?.focus()
  }

  function submit(event: SubmitEvent) {
    event.preventDefault()
    if (!draft.name.trim()) {
      error = 'Escribe un nombre para tu instancia.'
      return
    }
    if (draft.minRam > draft.maxRam) {
      openAdvanced()
      error = 'La RAM mínima no puede superar la máxima.'
      return
    }
    if (/(?:^|\s)-Xm[sx]\S*/i.test(draft.args)) {
      openAdvanced()
      error = 'Configura -Xms y -Xmx desde los campos de RAM, no en los argumentos.'
      return
    }
    onSave({ ...draft, name: draft.name.trim() })
  }

  function closeOnBackdrop(event: MouseEvent) {
    const dialog = event.currentTarget as HTMLDialogElement
    if (event.target !== dialog) return
    const rect = dialog.getBoundingClientRect()
    const outside =
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    if (outside) dialog.close()
  }

  const iconLabel = $derived(
    iconItems.find(([type]) => type === draft.type)?.[1] ?? 'Personalizado',
  )
</script>

<form class="instance-editor" use:revealInvalidFields onsubmit={submit}>
  <div class="editor-topline">
    <button type="button" class="text-link" onclick={onCancel}>
      ← Volver a Mis instancias
    </button>
    <span class="pill">CONFIGURACIÓN POR INSTANCIA</span>
  </div>

  <div class="page-heading">
    <div>
      <h1>{editing ? 'Configurar instancia' : 'Crear instancia'}</h1>
      <p>Configura el nombre, el icono y la versión de la instancia.</p>
    </div>
  </div>

  <div class="editor-sections">
      <section class="editor-section panel">
        <div class="editor-section-title">
          <div>
            <h3>Identidad y versión</h3>
            <p>Datos de identificación y versión de Minecraft.</p>
          </div>
        </div>

        <label>
          Nombre de la instancia
          <input
            use:autofocus
            required
            maxlength={60}
            bind:value={draft.name}
            placeholder="Nombre de la instancia"
          />
        </label>

        <div class="instance-icon-field">
          <div class="instance-icon-preview"><ItemLogo type={draft.type} /></div>
          <div>
            <span>Icono de la instancia</span>
            <strong>{iconLabel}</strong>
          </div>
          <button
            type="button"
            class="secondary"
            aria-haspopup="dialog"
            onclick={openIconPicker}
          >
            Cambiar icono
          </button>
        </div>

        <div class="form-row">
          <label>
            Versión de Minecraft
            <select bind:value={draft.version}>
              {#each mcVersions as version (version)}
                <option value={version}>{version}</option>
              {/each}
            </select>
          </label>
          <label>
            Mod loader
            <select bind:value={draft.loader}>
              {#each loaders as loader (loader)}
                <option value={loader}>{loader}</option>
              {/each}
            </select>
          </label>
        </div>

        <p class="field-help">
          Versiones y loaders de ejemplo. La compatibilidad real se validará en la
          aplicación de escritorio.
        </p>
      </section>

      <details class="editor-section panel advanced-options" bind:this={advancedRef}>
        <summary class="editor-section-title">
          <div>
            <h3>Opciones avanzadas</h3>
            <p>Memoria, ejecución y Java. Opcional para jugar.</p>
          </div>
          <svg
            class="advanced-chevron"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            aria-hidden="true"
          >
            <path d="m6 9 6 6 6-6" />
          </svg>
        </summary>

        <div class="advanced-content">
          <p class="advanced-notice field-help">
            Puedes dejar estos valores tal como están. Modifícalos solo si sabes qué
            necesita tu instancia; una configuración incorrecta puede impedir que el
            juego se inicie.
          </p>

          <h4 class="advanced-subheading">Memoria y ejecución</h4>
          <div class="form-row">
            <label>
              RAM mínima (GB)
              <input
                type="number"
                min="1"
                max="16"
                required
                value={draft.minRam}
                oninput={(e) => update('minRam', Number(e.currentTarget.value))}
              />
            </label>
            <label>
              RAM máxima (GB)
              <input
                type="number"
                min="1"
                max="16"
                required
                value={draft.maxRam}
                oninput={(e) => update('maxRam', Number(e.currentTarget.value))}
              />
            </label>
          </div>

          <div class="memory-preview">
            <span>Reserva máxima</span>
            <strong>{draft.maxRam} GB <small>/ 16 GB de referencia</small></strong>
            <progress value={draft.maxRam} max={16}></progress>
          </div>
          <p class="field-help">
            4 GB para Vanilla; 6–8 GB para modpacks. La memoria del equipo es
            ilustrativa.
          </p>

          <div class="form-row">
            <label>
              Java runtime
              <select bind:value={draft.java}>
                {#each javaRuntimes as runtime (runtime)}
                  <option value={runtime}>{runtime}</option>
                {/each}
              </select>
            </label>
            <label>
              Modo de compatibilidad
              <select bind:value={draft.compatibility}>
                {#each compatibilityModes as mode (mode)}
                  <option value={mode}>{mode}</option>
                {/each}
              </select>
            </label>
          </div>
          <p class="field-help">
            Los modos son propuestas de configuración; no alteran el sistema en esta
            maqueta.
          </p>

          <h4 class="advanced-subheading advanced-divider">Java y ventana</h4>
          <label>
            Argumentos adicionales de Java
            <textarea
              rows={4}
              spellcheck="false"
              bind:value={draft.args}
              placeholder="-XX:+UseG1GC"
            ></textarea>
          </label>
          <p class="field-help">
            Solo opciones JVM. Los argumentos -Xms y -Xmx se generan a partir de la
            RAM configurada.
          </p>

          <div class="form-row">
            <label>
              Ancho de ventana
              <input
                type="number"
                min="640"
                max="7680"
                required
                value={draft.width}
                oninput={(e) => update('width', Number(e.currentTarget.value))}
              />
            </label>
            <label>
              Alto de ventana
              <input
                type="number"
                min="480"
                max="4320"
                required
                value={draft.height}
                oninput={(e) => update('height', Number(e.currentTarget.value))}
              />
            </label>
          </div>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={draft.fullscreen} />
            Iniciar en pantalla completa
          </label>
        </div>
      </details>
    </div>

    <div class="instance-form-actions">
      {#if error}
        <p role="alert" class="form-error">{error}</p>
      {/if}
      <button class="primary" type="submit">
        {editing ? 'Guardar cambios' : 'Crear instancia'}
      </button>
      <button class="secondary" type="button" onclick={onCancel}>
        Cancelar
      </button>
    </div>

  <dialog
    bind:this={iconDialogRef}
    class="icon-picker-dialog modal panel"
    aria-labelledby="icon-picker-title"
    aria-describedby="icon-picker-description"
    onclick={closeOnBackdrop}
  >
    <div class="section-top">
      <h2 id="icon-picker-title">Elige un icono</h2>
      <button
        type="button"
        class="icon-button"
        aria-label="Cerrar selector de iconos"
        onclick={() => iconDialogRef?.close()}
      >
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          aria-hidden="true"
        >
          <path d="m6 6 12 12M6 18 18 6" />
        </svg>
      </button>
    </div>

    <p id="icon-picker-description" class="field-help">
      Selecciona un bloque u objeto para identificar la instancia.
    </p>

    <div class="item-picker">
      <div>
        {#each iconItems as [type, title] (type)}
          <button
            type="button"
            class={draft.type === type ? 'chosen' : ''}
            aria-pressed={draft.type === type}
            onclick={() => {
              update('type', type)
              iconDialogRef?.close()
            }}
          >
            <ItemLogo {type} />
            <span>{title}</span>
          </button>
        {/each}
      </div>
    </div>

    <p class="field-help">
      Selecciona un icono para aplicarlo. Cerrar sin elegir conserva el actual.
    </p>
  </dialog>
</form>