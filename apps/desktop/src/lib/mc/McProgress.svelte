<script lang="ts">
  import BitmapText from './BitmapText.svelte'
  import { stringWidth } from './font'
  import { posStyle, type Anchor } from './pos'

  interface Props {
    /** 0-100, o `null` si no se sabe cuanto falta. */
    percent: number | null
    label: string
    x?: number
    y?: number
    w?: number
    anchor?: Anchor
  }

  let { percent, label, x = -100, y = 0, w = 200, anchor = 'top' }: Props = $props()

  const pct = $derived(percent === null ? null : Math.max(0, Math.min(100, percent)))
  const fill = $derived(pct === null ? 0 : Math.floor((pct / 100) * (w - 2)))
  const percentText = $derived(pct === null ? '' : `${Math.floor(pct)}%`)
  const caption = $derived(pct === null ? `${label.replace(/[….]+$/, '')}...` : label)
</script>

<div class="mc-label" style={posStyle(x, y - 11, anchor)}><BitmapText text={caption} max={w} sr={false} /></div>
<div
  class="mc-progress"
  role="progressbar"
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={pct === null ? undefined : Math.floor(pct)}
  aria-label={label}
  style="{posStyle(x, y, anchor)};width:{w}px"
>
  {#if pct === null}
    <div class="indeterminate"></div>
  {:else}
    <div class="fill" style:width="{fill}px"></div>
  {/if}
</div>
{#if percentText}
  <div class="mc-label" style={posStyle(x + ((w - stringWidth(percentText)) >> 1), y + 1, anchor)}><BitmapText text={percentText} sr={false} /></div>
{/if}
