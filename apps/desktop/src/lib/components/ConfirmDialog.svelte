<script lang="ts">
  import Button from './Button.svelte'

  interface Props {
    open: boolean
    title: string
    message: string
    confirmLabel: string
    cancelLabel?: string
    onconfirm: () => void
    oncancel: () => void
  }

  let { open, title, message, confirmLabel, cancelLabel = 'Cancelar', onconfirm, oncancel }: Props = $props()

  let dialog = $state<HTMLDialogElement>()
  const uid = $props.id()

  // <dialog> modal: foco atrapado, Escape y fondo inerte los gestiona el navegador.
  $effect(() => {
    if (!dialog) return
    if (open && !dialog.open) dialog.showModal()
    if (!open && dialog.open) dialog.close()
  })
</script>

<dialog
  bind:this={dialog}
  class="modal panel"
  aria-labelledby="{uid}-title"
  aria-describedby="{uid}-msg"
  oncancel={(e) => {
    e.preventDefault()
    oncancel()
  }}
>
  <div class="section-top">
    <h2 id="{uid}-title">{title}</h2>
  </div>
  <p class="modal-text" id="{uid}-msg">{message}</p>
  <div class="modal-actions">
    {#if cancelLabel}
      <Button variant="ghost" onclick={oncancel}>{cancelLabel}</Button>
    {/if}
    <Button variant="primary" onclick={onconfirm}>{confirmLabel}</Button>
  </div>
</dialog>
