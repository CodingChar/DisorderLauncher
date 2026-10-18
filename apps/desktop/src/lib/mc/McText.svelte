<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { stringWidth, wrap } from './font'
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
  }

  let { text = '', x = 0, y = 0, anchor = 'top', w, color = '#ffffff', align = 'left', maxLines = 0, lineHeight = 10, role, sr = true }: Props = $props()

  const lines = $derived(text ? wrap(text, w - 1, maxLines) : [])
</script>

{#if text}
  {#if sr}<div class="sr-only" {role}>{text}</div>{/if}
  {#each lines as line, i (i)}
    {#if line}
      <div class="mc-label" style={posStyle(align === 'center' ? x + (w >> 1) - (stringWidth(line) >> 1) : x, y + i * lineHeight, anchor)}>
        <BitmapText text={line} {color} sr={false} />
      </div>
    {/if}
  {/each}
{/if}
