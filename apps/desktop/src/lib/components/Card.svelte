<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    /** `hero` = superficie principal; `plain` = tarjeta secundaria. */
    variant?: 'hero' | 'plain'
    children: Snippet
  }

  let { variant = 'plain', children }: Props = $props()
</script>

<section class="card {variant}">
  {@render children()}
</section>

<style>
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-soft);
    padding: 20px;
  }

  .plain {
    border-radius: var(--r-lg);
  }

  .hero {
    border-radius: 24px 0 24px 0;
    padding: 28px;
  }
</style>
