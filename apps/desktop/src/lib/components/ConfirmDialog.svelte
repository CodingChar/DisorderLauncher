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

<dialog bind:this={dialog} aria-labelledby="{uid}-title" aria-describedby="{uid}-msg" oncancel={(e) => { e.preventDefault(); oncancel() }}>
  <h3 id="{uid}-title">{title}</h3>
  <p id="{uid}-msg">{message}</p>
  <div class="actions">
    {#if cancelLabel}
      <Button variant="ghost" onclick={oncancel}>{cancelLabel}</Button>
    {/if}
    <Button variant="primary" onclick={onconfirm}>{confirmLabel}</Button>
  </div>
</dialog>

<style>
  dialog {
    border: 1px solid var(--border-strong);
    border-radius: var(--r-lg) 0 var(--r-lg) 0;
    background: var(--surface);
    color: var(--text-primary);
    padding: 24px;
    width: min(420px, calc(100vw - 32px));
    box-shadow: var(--shadow);
  }

  dialog::backdrop {
    background: rgba(0, 0, 0, 0.55);
  }

  h3 {
    font-size: 20px;
    margin-bottom: 8px;
  }

  p {
    margin: 0 0 20px;
    color: var(--text-secondary);
    line-height: 1.55;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
</style>
