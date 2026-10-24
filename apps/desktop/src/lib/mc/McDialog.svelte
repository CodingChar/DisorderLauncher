<script lang="ts">
  import { onMount } from 'svelte'
  import McButton from './McButton.svelte'
  import McList from './McList.svelte'
  import McText from './McText.svelte'
  import BitmapText from './BitmapText.svelte'
  import { wrap } from './font'
  import { playSfx } from '../audio/sfx'
  import { pushBack } from './back'
  import { useGui } from './gui'

  export interface DialogButton {
    label: string
    onclick: () => void
  }

  interface Props {
    title: string
    topAligned?: boolean
    highlight?: { x: number; y: number; w: number; h: number }
    message?: string
    buttons: DialogButton[]
    /** Esc (u otra salida sin elegir). */
    onclose: () => void
    /** Boton que recibe el foco al abrir (en acciones destructivas, el de cancelar). */
    cancelIndex?: number
    /** Mensaje largo: se muestra en una lista con barra de desplazamiento. */
    scroll?: boolean
    /** Dialogo de error: suena el aviso de «no se pudo» al abrirlo. */
    error?: boolean
  }

  let { topAligned = false, highlight, title, message = '', buttons, onclose, cancelIndex = buttons.length - 1, scroll = false, error = false }: Props = $props()

  const gui = useGui()
  const uid = $props.id()
  let layer = $state<HTMLDivElement>()

  const MSG_W = 280
  const bodyLines = $derived(scroll && message ? wrap(message, MSG_W - 14).map((text, id) => ({ id, text })) : [])
  // Alto del cuerpo: lineas de texto, o la banda con scroll (nunca mas de lo que cabe en pantalla).
  const bodyHeight = $derived(
    !message ? 0 : scroll ? Math.min(bodyLines.length * 10 + 8, gui.H - 96) : Math.min(wrap(message, MSG_W - 1, 8).length, Math.max(1, Math.floor((gui.H - 110) / 10))) * 10,
  )
  const lineCount = $derived(scroll ? 0 : bodyHeight / 10)
  // Bloque centrado: titulo (9) + hueco + cuerpo + hueco + botones (20).
  const total = $derived(9 + 8 + bodyHeight + (bodyHeight ? 14 : 0) + 20)
  const top = $derived(topAligned ? 16 : Math.max(14, (gui.H - total) >> 1))
  const messageY = $derived(top + 17)
  const buttonsY = $derived(top + 9 + 8 + bodyHeight + (bodyHeight ? 14 : 0))

  const layout = $derived.by(() => {
    const n = buttons.length
    if (n === 1) return [{ x: -100, w: 200 }]
    if (n === 2) return [{ x: -100, w: 98 }, { x: 2, w: 98 }]
    return [{ x: -100, w: 64 }, { x: -32, w: 64 }, { x: 36, w: 64 }]
  })

  // Esc cierra el dialogo antes que cualquier «Atras» de la pantalla de debajo.
  $effect(() => pushBack(() => onclose()))

  onMount(() => {
    if (error) playSfx('fail')
    const opener = document.activeElement as HTMLElement | null
    const stage = layer?.closest('.mc-stage')
    // Todo lo que rodea al dialogo (a cualquier nivel, hasta el escenario) queda inerte; sus ancestros no.
    const blocked: HTMLElement[] = []
    for (let node: HTMLElement | null = layer ?? null; node && node.parentElement && node !== stage; node = node.parentElement) {
      for (const sibling of node.parentElement.children) {
        if (sibling !== node && sibling instanceof HTMLElement && !sibling.inert) {
          sibling.inert = true
          blocked.push(sibling)
        }
      }
    }
    layer?.querySelectorAll<HTMLElement>('button')[Math.min(cancelIndex, buttons.length - 1)]?.focus()
    return () => {
      for (const el of blocked) el.inert = false
      opener?.focus?.()
    }
  })

  // Tab circula solo entre los botones del dialogo.
  function trap(event: KeyboardEvent) {
    if (event.key === 'Escape' && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault()
      event.stopPropagation()
      playSfx('back')
      onclose()
      return
    }
    if (event.key !== 'Tab' || !layer) return
    const list = [...layer.querySelectorAll<HTMLElement>('button:not(:disabled), [role="listbox"]')]
    if (list.length === 0) return
    const i = list.indexOf(document.activeElement as HTMLElement)
    const next = event.shiftKey ? (i <= 0 ? list.length - 1 : i - 1) : i === list.length - 1 ? 0 : i + 1
    event.preventDefault()
    list[next].focus()
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div bind:this={layer} class="mc-dialog-layer" style:background={highlight ? 'transparent' : undefined} role="dialog" aria-modal="true" aria-labelledby="{uid}-title" aria-describedby={message ? `${uid}-msg` : undefined} tabindex="-1" onkeydown={trap}>
  {#if highlight}
    <div aria-hidden="true" style="position:absolute;pointer-events:none;left:calc(var(--cx) * 1px + {highlight.x - 2}px);top:{highlight.y - 2}px;width:{highlight.w + 4}px;height:{highlight.h + 4}px;border:2px solid #ffff55;box-shadow:0 0 0 1000px rgba(0,0,0,.6);box-sizing:border-box"></div>
  {/if}
  <div class="mc-dialog-panel" aria-hidden="true" style="left:calc(var(--cx) * 1px - 150px);top:{top - 12}px;width:300px;height:{total + 24}px"></div>
  <span id="{uid}-title" class="sr-only">{title}</span>
  {#if message}<span id="{uid}-msg" class="sr-only">{message}</span>{/if}
  <div class="mc-label" style="left:calc(var(--cx) * 1px);top:{top}px" aria-hidden="true"><BitmapText text={title} center sr={false} /></div>
  {#if message && scroll}
    <McList
      items={bodyLines}
      getId={(l) => String(l.id)}
      row={(l) => ({ lines: [{ text: l.text, color: '#a0a0a0' }] })}
      x={-(MSG_W >> 1)}
      y={messageY}
      w={MSG_W}
      h={bodyHeight}
      rowHeight={10}
      selectable={false}
      ariaLabel={title}
    />
  {:else if message}
    <McText text={message} x={-(MSG_W >> 1)} y={messageY} w={MSG_W} align="center" color="#a0a0a0" maxLines={lineCount} sr={false} />
  {/if}
  {#each buttons as button, i (i)}
    <McButton label={button.label} x={layout[i].x} w={layout[i].w} y={buttonsY} onclick={button.onclick} />
  {/each}
</div>
