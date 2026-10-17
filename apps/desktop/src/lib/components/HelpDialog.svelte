<script lang="ts">
  import Button from './Button.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    open: boolean
    onclose: () => void
  }

  let { open, onclose }: Props = $props()

  let dialog = $state<HTMLDialogElement>()
  const uid = $props.id()

  $effect(() => {
    if (!dialog) return
    if (open && !dialog.open) dialog.showModal()
    if (!open && dialog.open) dialog.close()
  })

  /** Los clics en el `::backdrop` apuntan al propio dialogo: hay que descartar los que caen dentro. */
  function closeOnBackdrop(event: MouseEvent) {
    if (event.target !== event.currentTarget) return
    const rect = (event.currentTarget as HTMLDialogElement).getBoundingClientRect()
    const outside = event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom
    if (outside) onclose()
  }
</script>

<dialog
  bind:this={dialog}
  class="modal panel"
  aria-labelledby="{uid}-title"
  onclick={closeOnBackdrop}
  oncancel={(e) => {
    e.preventDefault()
    onclose()
  }}
>
  <div class="section-top">
    <h2 id="{uid}-title">Ayuda</h2>
    <button class="icon-button" aria-label="Cerrar" onclick={onclose}><Icon name="close" /></button>
  </div>

  <div class="help-content">
    <p>Bienvenido a DisorderLauncher.</p>
    <p class="subtle">
      En <strong>Inicio</strong> eliges tu instancia y pulsas JUGAR; ahí ves cómo avanza la preparación y lo que escribe el juego.
      En <strong>Mis instancias</strong> creas y configuras tus mundos.
    </p>
    <p class="subtle">
      En <strong>Perfiles</strong> eliges con qué nombre juegas, y en <strong>Skins</strong> guardas y editas el aspecto de tu personaje.
    </p>
    <p class="subtle">
      Puedes jugar sin conexión siempre que la versión ya esté descargada. Si algo falla, usa <strong>Reparar</strong> en Inicio o en Herramientas.
    </p>
    <Button variant="primary" onclick={onclose}>Entendido</Button>
  </div>
</dialog>
