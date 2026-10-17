<script lang="ts">
  import {
    defaults,
    type Instance,
    type Profile,
    type Session,
  } from './lib/types'
  import Avatar from './lib/Avatar.svelte'
  import Icon from './lib/Icon.svelte'
  import InstanceEditor from './lib/InstanceEditor.svelte'
  import ItemLogo from './lib/ItemLogo.svelte'
  import SessionMonitor from './lib/SessionMonitor.svelte'
  import SkinPreview from './lib/SkinPreview.svelte'

  // ---------------------------------------------------------------- datos

  const initialInstances = [
    {
      name: 'Mi mundo survival',
      version: '1.21.4',
      loader: 'Fabric',
      mods: 24,
      type: 'grass',
      time: 'Hace 2 horas',
    },
    {
      name: 'Vanilla, sin más',
      version: '1.21.4',
      loader: 'Vanilla',
      mods: 0,
      type: 'stone',
      time: 'Ayer',
    },
    {
      name: 'Better Adventures',
      version: '1.20.1',
      loader: 'Forge',
      mods: 86,
      type: 'sand',
      time: 'Hace 3 días',
    },
  ]

  const initialProfiles: Profile[] = [
    {
      id: 'p-alex',
      name: 'Alex',
      created: 'Hace 3 meses',
      skins: [
        {
          id: 's-alex-1',
          name: 'Constructor',
          head: '#a39576',
          body: '#71847b',
          arm: '#ab9875',
          leg: '#515c5a',
          equipped: true,
        },
        {
          id: 's-alex-2',
          name: 'Minero nocturno',
          head: '#6d6a63',
          body: '#4a5b76',
          arm: '#8c8375',
          leg: '#3a3f48',
          equipped: false,
        },
        {
          id: 's-alex-3',
          name: 'Arena',
          head: '#c0a878',
          body: '#b9ac87',
          arm: '#cbb992',
          leg: '#8a7c5e',
          equipped: false,
        },
      ],
    },
    {
      id: 'p-pollito',
      name: 'Pollito',
      created: 'Ayer',
      skins: [
        {
          id: 's-pollito-1',
          name: 'Explorador',
          head: '#8b9a65',
          body: '#5f7a52',
          arm: '#a3b18d',
          leg: '#40483a',
          equipped: true,
        },
      ],
    },
    { id: 'p-invitado', name: 'Invitado', created: 'Hace 2 semanas', skins: [] },
  ]

  /** Ideas de herramientas del launcher; ninguna ejecuta nada todavía. */
  const tools = [
    {
      id: 't-memoria',
      icon: 'memory',
      name: 'Probar memoria',
      description:
        'Calcula cuánta RAM conviene asignar a una instancia antes de lanzarla.',
      action: 'Probar',
    },
    {
      id: 't-cache',
      icon: 'trash',
      name: 'Limpiar caché',
      description: 'Borra descargas y archivos temporales que ya no estás usando.',
      action: 'Limpiar',
    },
    {
      id: 't-backup',
      icon: 'archive',
      name: 'Copia de seguridad',
      description:
        'Exporta tus instancias y su configuración a un archivo comprimido.',
      action: 'Exportar',
    },
    {
      id: 't-diagnostico',
      icon: 'clipboard',
      name: 'Informe de diagnóstico',
      description:
        'Resume tu sistema y las últimas sesiones para enviarlas a soporte.',
      action: 'Generar',
    },
    {
      id: 't-reparar',
      icon: 'zap',
      name: 'Reparar instancia',
      description: 'Verifica los archivos de una instancia y descarga lo que falte.',
      action: 'Reparar',
    },
    {
      id: 't-mods',
      icon: 'grid',
      name: 'Mover mods',
      description: 'Traslada mods entre instancias sin tocar la instalación base.',
      action: 'Abrir',
    },
  ]

  const explore = [
    {
      title: 'Better Adventures',
      type: 'grass',
      tag: 'AVENTURA',
      copy: 'Nuevos biomas, estructuras y aventuras por descubrir.',
    },
    {
      title: 'Un mundo más bonito',
      type: 'stone',
      tag: 'RENDIMIENTO',
      copy: 'Una experiencia fluida con mejoras visuales.',
    },
    {
      title: 'La esencia de Minecraft',
      type: 'sand',
      tag: 'VANILLA +',
      copy: 'Todo lo que te gusta, con un pequeño extra.',
    },
  ]

  const nav = [
    { name: 'Inicio', icon: 'home' },
    { name: 'Mis instancias', icon: 'box' },
    { name: 'Explorar', icon: 'compass' },
    { name: 'Herramientas', icon: 'wrench' },
    { name: 'Perfiles', icon: 'user' },
    { name: 'Skins', icon: 'shirt' },
  ]

  // --------------------------------------------------------------- estado

  let page = $state('Inicio')
  let blueTheme = $state(false)
  let closeLauncher = $state(false)
  let instances = $state<Instance[]>(
    initialInstances.map((v) => ({ ...defaults, ...v })),
  )
  let selected = $state(0)
  let profiles = $state<Profile[]>(initialProfiles)
  let activeProfileId = $state(initialProfiles[0].id)
  let session = $state<Session | null>(null)
  let editor = $state<{ index: number | null; initial: Instance } | null>(null)
  let modal = $state<{
    kind: 'profile' | 'help'
    title: string
    profileId?: string
  } | null>(null)
  let modalRef = $state<HTMLDialogElement>()
  let name = $state('')
  let profileError = $state('')

  // `SessionMonitor` se desmonta al salir de Inicio, así que el estado de "consola
  // limpiada" vive aquí para que sobreviva a la navegación.
  let clearedSession = $state<Session | null>(null)

  type ModalState = NonNullable<typeof modal>

  /**
   * El estado y el DOM no pueden gobernarse mutuamente: si un efecto cerrara y
   * abriera el `<dialog>` según `modal`, volvería a abrirlo en cuanto cualquier
   * cierre (Escape, backdrop) dejara `open = false`, y el evento `close` no
   * llegaría a vaciar `modal`. Por eso solo se abre desde el efecto, y el cierre es
   * siempre explícito.
   */
  function openModal(next: ModalState) {
    modal = next
  }

  function closeModal() {
    modalRef?.close()
    modal = null
  }

  $effect(() => {
    if (modal && modalRef && !modalRef.open) modalRef.showModal()
  })

  /**
   * Escape se atiende aquí en vez de confiar en el evento `close` del navegador:
   * así el cierre es determinista y no depende de que ese evento se despache.
   */
  function onModalKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return
    event.preventDefault()
    closeModal()
  }

  /** Los clics en el `::backdrop` apuntan al propio diálogo: hay que descartarlos. */
  function closeModalOnBackdrop(event: MouseEvent) {
    const dialog = event.currentTarget as HTMLDialogElement
    if (event.target !== dialog) return
    const rect = dialog.getBoundingClientRect()
    const outside =
      event.clientX < rect.left ||
      event.clientX > rect.right ||
      event.clientY < rect.top ||
      event.clientY > rect.bottom
    if (outside) closeModal()
  }

  const activeProfile = $derived(
    profiles.find((p) => p.id === activeProfileId) ?? profiles[0],
  )

  /** Sustituye al atributo `autofocus`, que Svelte marca como mala práctica. */
  function autofocus(node: HTMLElement) {
    node.focus()
  }

  function go(to: string) {
    editor = null
    page = to
  }

  function editInstance(index: number | null) {
    page = 'Mis instancias'
    editor = {
      index,
      initial: index === null ? { ...defaults } : { ...instances[index] },
    }
  }

  function startInstance(index: number) {
    const instance = { ...instances[index] }
    const started = new Date().toLocaleTimeString('es-ES')
    const skin = activeProfile.skins.find((s) => s.equipped)
    selected = index
    clearedSession = null
    session = {
      instance,
      profile: activeProfile.name,
      started,
      logs: [
        `[${started}] [INFO] [DisorderLauncher] Inicio de sesión de demostración.`,
        `[${started}] [INFO] Instancia: ${instance.name}`,
        `[${started}] [INFO] Perfil local: ${activeProfile.name}`,
        `[${started}] [INFO] Skin: ${skin ? skin.name : '(ninguna equipada)'}`,
        `[${started}] [INFO] Minecraft ${instance.version} · ${instance.loader} · ${instance.mods} mods`,
        `[${started}] [INFO] Java: ${instance.java} · Compatibilidad: ${instance.compatibility}`,
        `[${started}] [INFO] Memoria: -Xms${instance.minRam}G -Xmx${instance.maxRam}G`,
        `[${started}] [INFO] JVM: ${instance.args || '(sin argumentos adicionales)'}`,
        `[${started}] [WARN] Modo maqueta: no se han descargado ni ejecutado archivos.`,
        `[${started}] [INFO] Vista de sesión preparada. Esperando integración de escritorio.`,
      ],
    }
    instances = instances.map((item, i) =>
      i === index ? { ...item, time: 'Ahora' } : item,
    )
    page = 'Inicio'
  }

  function openProfileModal(profileId: string) {
    const target = profiles.find((p) => p.id === profileId) ?? activeProfile
    name = target.name
    profileError = ''
    openModal({ kind: 'profile', title: 'Editar perfil', profileId: target.id })
  }

  function saveProfileName() {
    const trimmed = name.trim()
    const targetId = modal?.profileId ?? activeProfileId
    if (!trimmed) {
      profileError = 'Escribe un nombre para el perfil.'
      return
    }
    // Los perfiles se identifican por nombre en la interfaz, así que dos nombres
    // iguales harían la lista ambigua. Se compara sin distinguir mayúsculas para
    // no dejar pasar "Alex" junto a "alex".
    const taken = profiles.some(
      (p) =>
        p.id !== targetId && p.name.trim().toLowerCase() === trimmed.toLowerCase(),
    )
    if (taken) {
      profileError = `Ya existe un perfil llamado "${trimmed}".`
      return
    }
    profiles = profiles.map((p) =>
      p.id === targetId ? { ...p, name: trimmed } : p,
    )
    closeModal()
  }

  function equipSkin(profileId: string, skinId: string) {
    profiles = profiles.map((p) =>
      p.id === profileId
        ? {
            ...p,
            skins: p.skins.map((s) => ({ ...s, equipped: s.id === skinId })),
          }
        : p,
    )
  }

  function saveInstance(value: Instance, index: number | null) {
    if (index === null) {
      instances = [...instances, value]
    } else {
      instances = instances.map((item, i) => (i === index ? value : item))
    }
    editor = null
    page = 'Mis instancias'
  }

  const skinCount = (profile: Profile) =>
    `${profile.skins.length} ${profile.skins.length === 1 ? 'skin' : 'skins'}`

  const pageCopy: Record<
    string,
    () => { eyebrow: string; description: string }
  > = {
    Inicio: () => ({
      eyebrow: 'ESTADO DE LA SESIÓN',
      description:
        'Información de la instancia activa y registro de ejecución.',
    }),
    'Mis instancias': () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: 'Creación, configuración y ejecución de instancias.',
    }),
    Explorar: () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: 'Catálogo de modpacks de ejemplo.',
    }),
    Herramientas: () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: 'Utilidades del launcher. De momento solo son ideas.',
    }),
    Perfiles: () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: 'Crea perfiles locales y elige cuál está activo.',
    }),
    Skins: () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: `Skins del perfil ${activeProfile.name}. Cada perfil tiene las suyas.`,
    }),
    Ajustes: () => ({
      eyebrow: 'DISORDERLAUNCHER',
      description: 'Preferencias generales del launcher.',
    }),
  }

  const copy = $derived((pageCopy[page] ?? pageCopy['Mis instancias'])())
</script>

<div class="app {blueTheme ? 'blue' : ''}">
  <aside class="sidebar">
    <button type="button" class="brand" onclick={() => go('Inicio')}>
      <div>disorder<span>LAUNCHER</span></div>
    </button>

    <div class="workspace-label">TU ESPACIO DE JUEGO</div>

    <nav>
      {#each nav as item (item.name)}
        <button
          class="nav-item {page === item.name ? 'active' : ''}"
          onclick={() => go(item.name)}
        >
          <Icon name={item.icon} />
          {item.name}
          {#if item.name === 'Mis instancias'}
            <small>{String(instances.length).padStart(2, '0')}</small>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="sidebar-bottom">
      <div class="offline-note">
        <span class="status-dot"></span>
        <span>
          Modo sin conexión<small>Perfil local disponible</small>
        </span>
      </div>

      <button
        class="nav-item {page === 'Ajustes' ? 'active' : ''}"
        onclick={() => go('Ajustes')}
      >
        <Icon name="settings" />
        Ajustes
      </button>

      <button
        class="nav-item"
        onclick={() => openModal({ kind: 'help', title: 'Ayuda' })}
      >
        <Icon name="help" />
        Ayuda y soporte
        <Icon name="arrow" size={16} />
      </button>

      <button
        class="sidebar-profile"
        onclick={() => openProfileModal(activeProfileId)}
      >
        <Avatar />
        <span>
          {activeProfile.name}
          <small>Perfil activo</small>
        </span>
        <Icon name="chevron" size={16} />
      </button>

      <div class="build-version">
        <span>DisorderLauncher</span>
        <span>v0.1.0 <b>ALPHA</b></span>
      </div>
    </div>
  </aside>

  <main>
    <header>
      <div class="breadcrumb">
        Launcher <span>/</span> <strong>{page}</strong>
      </div>
      <div class="header-actions">
        <span class="connection">
          <span class="status-dot"></span>
          <span class="connection-label">Todos los sistemas listos</span>
        </span>
        <span class="header-divider"></span>
        <button
          class="icon-button"
          onclick={() => (blueTheme = !blueTheme)}
          aria-label={blueTheme
            ? 'Cambiar a tema negro'
            : 'Cambiar a tema azul oscuro'}
        >
          <Icon name="moon" size={19} />
        </button>
        <button
          class="mini-profile"
          onclick={() => openProfileModal(activeProfileId)}
          aria-label="Abrir perfil"
        >
          <Avatar />
        </button>
      </div>
    </header>

    <div class="content">
      {#if editor}
        {#key editor.index ?? 'new'}
          <InstanceEditor
            initial={editor.initial}
            editing={editor.index !== null}
            onCancel={() => (editor = null)}
            onSave={(value) => saveInstance(value, editor?.index ?? null)}
          />
        {/key}
      {:else}
        <div class="page-heading">
          <div>
            <div class="eyebrow">{copy.eyebrow}</div>
            <h1>{page === 'Inicio' ? 'Centro de sesión' : page}</h1>
            <p>{copy.description}</p>
          </div>
          {#if page === 'Mis instancias'}
            <button
              class="secondary new-instance"
              onclick={() => editInstance(null)}
            >
              <Icon name="plus" size={17} />
              Nueva instancia
            </button>
          {/if}
        </div>

        {#if page === 'Inicio'}
          <SessionMonitor
            {session}
            {clearedSession}
            goInstances={() => go('Mis instancias')}
            onClearConsole={(value) => (clearedSession = value)}
          />
        {/if}

        {#if page === 'Mis instancias'}
          <div class="library-toolbar">
            <span>{instances.length} instancias en tu biblioteca</span>
            <span class="tiny-label">CONFIGURACIONES INDEPENDIENTES</span>
          </div>

          <div class="instance-grid library-grid">
            {#each instances as instance, index (index)}
              <section
                class="instance-card library-card {session && selected === index
                  ? 'selected'
                  : ''}"
              >
                <div class="instance-top">
                  <ItemLogo type={instance.type} />
                  <span class="loader-badge">{instance.loader}</span>
                  {#if session && selected === index}
                    <span class="active-instance">
                      <span class="status-dot"></span>
                      En ejecución
                    </span>
                  {/if}
                </div>

                <h4>{instance.name}</h4>
                <p>
                  Minecraft {instance.version}
                  <span>·</span>
                  {instance.mods} mods
                </p>

                <div class="library-specs">
                  <span>{instance.maxRam} GB RAM</span>
                  <span>{instance.compatibility}</span>
                </div>

                <div class="library-actions">
                  <button class="secondary" onclick={() => editInstance(index)}>
                    <Icon name="settings" size={15} />
                    Configurar
                  </button>
                  {#if session && selected === index}
                    <button class="secondary" onclick={() => (session = null)}>
                      Detener demo
                    </button>
                  {:else}
                    <button
                      class="primary"
                      disabled={!!session}
                      onclick={() => startInstance(index)}
                    >
                      <Icon name="play" size={16} />
                      Jugar
                    </button>
                  {/if}
                </div>

                <div class="instance-bottom">
                  <span>
                    <span class="mini-clock"></span>
                    {instance.time}
                  </span>
                  <span>{instance.java}</span>
                </div>
              </section>
            {/each}
          </div>

          {#if session}
            <p class="library-notice">
              Hay una sesión de demostración activa. Detén esa sesión antes de iniciar
              otra.
            </p>
          {/if}
        {/if}

        {#if page === 'Explorar'}
          <div class="explore-grid">
            {#each explore as item, i (item.title)}
              <section class="explore-card panel">
                <div class="explore-art art-{i}">
                  <ItemLogo type={item.type} />
                </div>
                <div class="explore-copy">
                  <span class="eyebrow">{item.tag}</span>
                  <h2>{item.title}</h2>
                  <p>{item.copy}</p>
                  <button class="secondary" onclick={() => go('Mis instancias')}>
                    Ir a Mis instancias
                    <Icon name="arrow" size={16} />
                  </button>
                  <small>Catálogo de ejemplo</small>
                </div>
              </section>
            {/each}
          </div>
        {/if}

        {#if page === 'Herramientas'}
          <div class="library-toolbar">
            <span>{tools.length} herramientas</span>
            <span class="tiny-label">SOLO IDEAS · SIN ACCIÓN REAL</span>
          </div>

          <div class="tool-grid">
            {#each tools as tool (tool.id)}
              <section class="tool-card panel">
                <div class="tool-icon"><Icon name={tool.icon} size={22} /></div>
                <h3>{tool.name}</h3>
                <p>{tool.description}</p>
                <div class="tool-foot">
                  <span class="tiny-label">SOLO IDEA</span>
                  <button class="secondary" disabled>{tool.action}</button>
                </div>
              </section>
            {/each}
          </div>

          <p class="library-notice">
            Esta sección reserva el sitio de las utilidades del launcher. Las tarjetas
            son propuestas de diseño: todavía no ejecutan nada ni tocan los archivos de
            tu equipo.
          </p>
        {/if}

        {#if page === 'Perfiles'}
          <section class="settings-panel panel">
            <div class="section-title">
              <h3>Perfiles locales</h3>
              <span>Solo uno puede estar activo a la vez</span>
            </div>

            {#each profiles as profile (profile.id)}
              <div
                class="account-row {profile.id === activeProfileId
                  ? 'active'
                  : ''}"
              >
                <Avatar />
                <div>
                  <h3>{profile.name}</h3>
                  <p class="subtle">
                    {profile.id === activeProfileId
                      ? `Perfil activo · ${skinCount(profile)}`
                      : `${skinCount(profile)} · Creado ${profile.created.toLowerCase()}`}
                  </p>
                </div>
                <div class="account-actions">
                  {#if profile.id === activeProfileId}
                    <span class="pill">
                      <span class="status-dot"></span>
                      ACTIVO
                    </span>
                  {:else}
                    <button
                      class="secondary"
                      onclick={() => {
                        activeProfileId = profile.id
                        page = 'Skins'
                      }}
                    >
                      Activar
                    </button>
                  {/if}
                  <button
                    class="secondary"
                    onclick={() => openProfileModal(profile.id)}
                  >
                    Renombrar
                  </button>
                </div>
              </div>
            {/each}

            <div class="account-row">
              <Icon name="grid" size={30} />
              <div>
                <h3>Microsoft</h3>
                <p class="subtle">
                  Integración prevista para una próxima versión.
                </p>
              </div>
              <span class="pill">PRÓXIMAMENTE</span>
            </div>

            <p class="library-notice">
              El perfil activo determina las skins disponibles y el nombre que aparece
              al iniciar una instancia.
            </p>
          </section>
        {/if}

        {#if page === 'Skins'}
          <div class="library-toolbar">
            <span>{skinCount(activeProfile)} de {activeProfile.name}</span>
            <span class="tiny-label">RELATIVAS AL PERFIL ACTIVO</span>
          </div>

          {#if activeProfile.skins.length === 0}
            <p class="library-notice">
              {activeProfile.name} todavía no tiene skins. Las skins pertenecen a un
              perfil, así que no se comparten entre perfiles.
            </p>
          {:else}
            <div class="instance-grid library-grid skin-grid">
              {#each activeProfile.skins as skin (skin.id)}
                <section
                  class="profile-card panel {skin.equipped ? 'selected' : ''}"
                >
                  <div class="profile-content">
                    <div class="skin-scene"><SkinPreview {skin} /></div>
                    <div>
                      <h4>{skin.name}</h4>
                      <span class="profile-status">
                        <span class="status-dot"></span>
                        {skin.equipped ? 'EQUIPADA' : 'DISPONIBLE'}
                      </span>
                    </div>
                  </div>
                  <small class="profile-caption">
                    Guardada en {activeProfile.name}. No aparece en otros perfiles.
                  </small>
                  <button
                    class="profile-manage"
                    disabled={skin.equipped}
                    onclick={() => equipSkin(activeProfileId, skin.id)}
                  >
                    {skin.equipped ? 'Equipada ahora' : 'Equipar skin'}
                    {#if !skin.equipped}<Icon name="arrow" size={14} />{/if}
                  </button>
                </section>
              {/each}
            </div>
          {/if}

          <p class="library-notice">
            Cambia de perfil desde
            <button class="text-link" onclick={() => go('Perfiles')}>Perfiles</button>
            para ver sus propias skins.
          </p>
        {/if}

        {#if page === 'Ajustes'}
          <section class="settings-panel panel">
            <h3>Preferencias del launcher</h3>
            <div class="preference">
              <span>
                Cerrar launcher al jugar
                <small>Preferencia visual de esta maqueta.</small>
              </span>
              <button
                role="switch"
                aria-checked={closeLauncher}
                aria-label="Cerrar al jugar"
                class="switch {closeLauncher ? 'on' : ''}"
                onclick={() => (closeLauncher = !closeLauncher)}
              >
                <i></i>
              </button>
            </div>
            <div class="settings-row">
              <div>
                <strong>Apariencia</strong>
                <p class="subtle">Selecciona el tema de la interfaz.</p>
              </div>
              <button
                class="secondary"
                onclick={() => (blueTheme = !blueTheme)}
              >
                <Icon name="moon" />
                {blueTheme ? 'Cambiar a negro' : 'Cambiar a azul oscuro'}
              </button>
            </div>
          </section>
        {/if}
      {/if}

      <footer>
        <span>
          <span class="status-dot"></span>
          Puedes jugar sin conexión
        </span>
        <span>
          DisorderLauncher · Prototipo
          <span class="footer-mark">d.</span>
        </span>
      </footer>
    </div>
  </main>

  {#if modal}
    <dialog
      bind:this={modalRef}
      class="modal panel"
      aria-label={modal.title}
      onclick={closeModalOnBackdrop}
      onkeydown={onModalKeydown}
      onclose={closeModal}
    >
      <div class="section-top">
          <h2>{modal.title}</h2>
          <button
            class="icon-button"
            aria-label="Cerrar"
            onclick={closeModal}
          >
            <Icon name="close" />
          </button>
        </div>

        {#if modal.kind === 'profile'}
          <form
            onsubmit={(e) => {
              e.preventDefault()
              saveProfileName()
            }}
          >
            <div class="account-row">
              <Avatar />
              <div>
                <h3>{profiles.find((p) => p.id === modal?.profileId)?.name}</h3>
                <p class="subtle">
                  {modal.profileId === activeProfileId
                    ? 'Offline Gaming · Activo'
                    : 'Offline Gaming'}
                </p>
              </div>
              <Icon name="check" />
            </div>

            <label>
              Nombre del perfil local
              <input
                use:autofocus
                required
                maxlength={16}
                pattern={'[A-Za-z0-9_]{1,16}'}
                title="Entre 1 y 16 letras, números o guiones bajos"
                aria-invalid={profileError ? 'true' : undefined}
                aria-describedby={profileError ? 'profile-error' : undefined}
                bind:value={name}
                oninput={() => (profileError = '')}
                placeholder="Tu nombre en el juego"
              />
            </label>

            {#if profileError}
              <p role="alert" class="form-error" id="profile-error">
                {profileError}
              </p>
            {/if}

            <p class="subtle">
              Este nombre solo cambia el perfil. Sus skins se conservan.
            </p>
            <button class="primary" type="submit">Guardar perfil</button>
          </form>
        {:else}
          <div class="help-content">
            <p>Bienvenido a la maqueta de DisorderLauncher.</p>
            <p class="subtle">
              Crea, configura e inicia instancias desde Mis instancias. En Inicio
              puedes consultar la sesión y sus logs simulados.
            </p>
            <p class="subtle">
              En Perfiles eliges el perfil activo; en Skins gestionas las skins, que
              pertenecen a ese perfil.
            </p>
            <p class="subtle">
              Esta vista es un prototipo interactivo: no instala Minecraft ni conecta
              cuentas reales.
            </p>
            <button class="primary" onclick={closeModal}>
              Entendido
            </button>
          </div>
        {/if}
    </dialog>
  {/if}
</div>