<script lang="ts">
  interface Props {
    /** 0-100; `null` = progreso indeterminado. */
    percent: number | null
    label: string
  }

  let { percent, label }: Props = $props()
  const clamped = $derived(percent === null ? null : Math.max(0, Math.min(100, Math.round(percent))))
</script>

<div
  class="progress"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={clamped ?? undefined}
>
  <div class="progress-fill" class:indeterminate={clamped === null} style:width={clamped === null ? undefined : `${clamped}%`}></div>
</div>
