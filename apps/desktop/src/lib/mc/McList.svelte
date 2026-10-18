<script lang="ts" generics="T">
  import { untrack } from 'svelte'
  import BitmapText from './BitmapText.svelte'
  import McText from './McText.svelte'
  import type { RowIconLayer, RowSpec } from './listTypes'
  import {
    LIST_PAD,
    SCROLLBAR_W,
    clampScroll,
    ensureVisible,
    lineOffsets,
    maxScroll,
    moveSelection,
    scrollFromThumb,
    thumbGeometry,
    visibleRange,
    wheelScroll,
    type ListKey,
  } from './logic'

  interface Props {
    items: T[]
    getId: (item: T) => string
    /** Describe como se dibuja una fila. */
    row: (item: T) => RowSpec
    selectedId?: string | null
    x: number
    y: number
    w: number
    h: number
    rowHeight?: number
    /** Doble clic o Intro sobre una fila. */
    onactivate?: (item: T) => void
    onselect?: (item: T) => void
    /** Texto cuando no hay filas. */
    empty?: string
    loading?: boolean
    loadingText?: string
    ariaLabel: string
    /** `false`: la lista solo se desplaza (consola). */
    selectable?: boolean
    /** Mantiene el final a la vista mientras lleguen filas, salvo que la persona suba. */
    followEnd?: boolean
  }

  let {
    items,
    getId,
    row,
    selectedId = $bindable(null),
    x,
    y,
    w,
    h,
    rowHeight = 36,
    onactivate,
    onselect,
    empty = '',
    loading = false,
    loadingText = 'Cargando...',
    ariaLabel,
    selectable = true,
    followEnd = false,
  }: Props = $props()

  const uid = $props.id()
  let scroll = $state(0)
  let stuck = $state(true)
  let box = $state<HTMLDivElement>()
  let dragStart: { pointerY: number; thumbTop: number } | null = null

  const count = $derived(items.length)
  const rowWidth = $derived(w - 10)
  const range = $derived(visibleRange(count, rowHeight, h, scroll))
  const thumb = $derived(thumbGeometry(count, rowHeight, h, scroll))
  const selectedIndex = $derived(selectedId == null ? -1 : items.findIndex((item) => getId(item) === selectedId))
  const pageRows = $derived(Math.max(1, Math.floor(h / rowHeight)))
  const visible = $derived.by(() => {
    const out: { item: T; index: number; id: string; spec: RowSpec }[] = []
    for (let i = range.first; i <= range.last; i++) {
      const item = items[i]
      out.push({ item, index: i, id: getId(item), spec: row(item) })
    }
    return out
  })

  function setScroll(next: number) {
    scroll = clampScroll(next, count, rowHeight, h)
    stuck = scroll >= maxScroll(count, rowHeight, h)
  }

  // Cambia la cantidad de filas: se corrige el scroll y, en la consola, se sigue el final.
  $effect(() => {
    const n = count
    const height = h
    untrack(() => {
      if (followEnd && stuck) scroll = maxScroll(n, rowHeight, height)
      else scroll = clampScroll(scroll, n, rowHeight, height)
    })
  })

  // La fila seleccionada siempre queda entera a la vista.
  $effect(() => {
    const index = selectedIndex
    if (index < 0 || !selectable) return
    untrack(() => {
      scroll = ensureVisible(index, count, rowHeight, h, scroll)
    })
  })

  // `wheel` debe ser no pasivo para poder impedir el scroll de la pagina.
  $effect(() => {
    const el = box
    if (!el) return
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      setScroll(scroll + wheelScroll(event.deltaY, rowHeight))
    }
    el.addEventListener('wheel', onWheel, { passive: false })
    return () => el.removeEventListener('wheel', onWheel)
  })

  function choose(index: number) {
    const item = items[index]
    if (!item) return
    selectedId = getId(item)
    onselect?.(item)
  }

  function onKey(event: KeyboardEvent) {
    if (event.ctrlKey || event.metaKey || event.altKey) return
    const keys: ListKey[] = ['ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 'Home', 'End']
    if (event.key === 'Enter' && selectable && selectedIndex >= 0) {
      event.preventDefault()
      onactivate?.(items[selectedIndex])
      return
    }
    if (!keys.includes(event.key as ListKey)) return
    event.preventDefault()
    const key = event.key as ListKey
    if (selectable) {
      const next = moveSelection(selectedIndex, key, count, pageRows)
      if (next >= 0) choose(next)
      return
    }
    if (key === 'Home') setScroll(0)
    else if (key === 'End') setScroll(maxScroll(count, rowHeight, h))
    else {
      const delta = key === 'ArrowDown' ? rowHeight : key === 'ArrowUp' ? -rowHeight : (key === 'PageDown' ? 1 : -1) * pageRows * rowHeight
      setScroll(scroll + delta)
    }
  }

  function startDrag(event: PointerEvent) {
    if (!thumb) return
    event.preventDefault()
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
    dragStart = { pointerY: event.clientY, thumbTop: thumb.top }
  }

  function drag(event: PointerEvent) {
    if (!dragStart || !box) return
    const s = box.getBoundingClientRect().height / h || 1
    setScroll(scrollFromThumb(dragStart.thumbTop + (event.clientY - dragStart.pointerY) / s, count, rowHeight, h))
  }

  function layerStyle(layer: RowIconLayer): string {
    const base = `background-image:url(${layer.src})`
    if (!layer.crop || !layer.imageWidth) return `${base};background-size:100% 100%`
    const [cx, cy, cw] = layer.crop
    const k = 32 / cw
    return `${base};background-size:${layer.imageWidth * k}px auto;background-position:${-cx * k}px ${-cy * k}px`
  }

  const layers = (icon: RowSpec['icon']): RowIconLayer[] => (icon ? (Array.isArray(icon) ? icon : [icon]) : [])
  const optionText = (spec: RowSpec) => spec.lines.map((l) => l.text).filter(Boolean).join('. ')
</script>

<div
  bind:this={box}
  class="mc-list"
  role="listbox"
  tabindex="0"
  aria-label={ariaLabel}
  aria-activedescendant={selectable && selectedIndex >= 0 ? `${uid}-opt-${selectedIndex}` : undefined}
  aria-busy={loading || undefined}
  style="left:calc(var(--cx) * 1px + {x}px);top:{y}px;width:{w}px;height:{h}px"
  onkeydown={onKey}
>
  <div class="mc-list-inner" style:transform="translateY({-scroll}px)">
    {#each visible as { item, index, id, spec } (id)}
      {@const icon = layers(spec.icon)}
      {@const textX = icon.length ? 37 : 5}
      {@const offsets = lineOffsets(rowHeight, spec.lines.length)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div
        class="mc-row"
        class:selected={selectable && selectedIndex === index}
        class:disabled={spec.disabled}
        style="top:{LIST_PAD + index * rowHeight}px;width:{rowWidth}px;height:{rowHeight}px"
        onclick={() => selectable && choose(index)}
        ondblclick={() => selectable && !spec.disabled && onactivate?.(item)}
      >
        {#if icon.length}
          <div class="mc-row-icon" aria-hidden="true">
            {#each icon as layer, i (i)}<div class="layer" style={layerStyle(layer)}></div>{/each}
          </div>
        {/if}
        {#each spec.lines as line, i (i)}
          {#if line.text}
            <div class="mc-row-text" style="left:{textX}px;top:{offsets[i]}px"><BitmapText text={line.text} color={line.color ?? (i === 0 ? '#ffffff' : '#808080')} max={rowWidth - textX - 2} sr={false} /></div>
          {/if}
        {/each}
      </div>
    {/each}
  </div>

  {#each visible as { index, id, spec } (id)}
    <div role="option" id="{uid}-opt-{index}" class="sr-only" aria-selected={selectable ? selectedIndex === index : undefined} aria-posinset={index + 1} aria-setsize={count}>{optionText(spec)}</div>
  {/each}

  {#if thumb}
    <div class="mc-scrollbar" aria-hidden="true">
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="mc-thumb" style="top:{thumb.top}px;height:{thumb.length}px" onpointerdown={startDrag} onpointermove={drag} onpointerup={() => (dragStart = null)} onpointercancel={() => (dragStart = null)}></div>
    </div>
  {/if}
</div>

{#if loading}
  <McText text={loadingText} x={x + 4} y={y + (h >> 1) - 5} w={w - 8} align="center" color="#808080" role="status" />
{:else if count === 0 && empty}
  <McText text={empty} x={x + 8} y={y + Math.max(4, (h >> 1) - 15)} w={w - 16} align="center" color="#808080" maxLines={4} />
{/if}
