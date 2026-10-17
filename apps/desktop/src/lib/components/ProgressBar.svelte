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
  class="bar"
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={clamped ?? undefined}
>
  <div class="fill" class:indeterminate={clamped === null} style:width={clamped === null ? undefined : `${clamped}%`}></div>
</div>

<style>
  .bar {
    height: 8px;
    border-radius: 999px;
    background: var(--surface-raised);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--action);
    border-radius: 999px;
    transition: width 0.2s;
  }

  .indeterminate {
    width: 35%;
    animation: slide 1.2s ease-in-out infinite alternate;
  }

  @keyframes slide {
    from {
      margin-left: 0;
    }
    to {
      margin-left: 65%;
    }
  }
</style>
