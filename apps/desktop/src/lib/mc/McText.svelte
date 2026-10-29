<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import McMore from './McMore.svelte'
  import { detail } from './detail.svelte'
  import { MORE_WIDTH, fitText, stringWidth, wrap } from './font'
  import { posStyle, type Anchor } from './pos'

  interface Props {
    text?: string
    /** Borde izquierdo de la caja, desde el centro horizontal. */
    x?: number
    y?: number
    anchor?: Anchor
    /** Ancho maximo de la caja. */
    w: number
    color?: string
    align?: 'left' | 'center'
    /** 0 = sin limite; si sobra texto, la ultima linea termina en «...». */
    maxLines?: number
    lineHeight?: number
    /** `alert` para errores y `status` para avisos: los lectores de pantalla los anuncian. */
    role?: 'alert' | 'status'
    /** Texto accesible oculto (desactivalo si ya se anuncia por otra via). */
    sr?: boolean
    /** Si el texto no cabe en `maxLines`, muestra «[+]» para leerlo entero en una ventana. */
    expandable?: boolean
    /** Titulo de esa ventana (por defecto segun el tipo de aviso). */
    detailTitle?: string
  }

  let { text = '', x = 0, y = 0, anchor = 'top', w, color = '#ffffff', align = 'left', maxLines = 0, lineHeight = 10, role, sr = true, expandable = true, detailTitle }: Props = $props()

  const full = $derived(text ? wrap(text, w - 1) : [])
  const truncated = $derived(expandable && maxLines > 0 && full.length > maxLines)
  // Con texto acortado, la ultima linea deja sitio a la marca «[+]».
  const lines = $derived.by(() => {
    if (!text) return []
    if (!truncated) return wrap(text, w - 1, maxLines)
    return [...full.slice(0, maxLines - 1), fitText(full.slice(maxLines - 1).join(' '), w - 1 - MORE_WIDTH)]
  })
  const lastWidth = $derived(stringWidth(lines[lines.length - 1] ?? ''))
  const lineX = (line: string) => (align === 'center' ? x + (w >> 1) - (stringWidth(line) >> 1) : x)
  const moreStyle = $derived(posStyle(lineX(lines[lines.length - 1] ?? '') + lastWidth + 2, y + (lines.length - 1) * lineHeight - 1, anchor))
  const title = $derived(detailTitle ?? (role === 'alert' ? 'Aviso importante' : role === 'status' ? 'Aviso' : 'Detalles'))
</script>

{#if text}
  {#if sr}<div class="sr-only" {role}>{text}</div>{/if}
  {#each lines as line, i (i)}
    {#if line}
      <div class="mc-label" style={posStyle(lineX(line), y + i * lineHeight, anchor)}>
        <BitmapText text={line} {color} sr={false} />
      </div>
    {/if}
  {/each}
  {#if truncated}
    <McMore style={moreStyle} label="Ver el texto completo" onclick={() => detail.show(title, text)} />
  {/if}
{/if}
