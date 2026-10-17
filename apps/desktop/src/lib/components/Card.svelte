<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    /** `hero` = superficie principal; `plain` = tarjeta secundaria. */
    variant?: 'hero' | 'plain'
    children: Snippet
  }

  let { variant = 'plain', children }: Props = $props()
</script>

<section class="panel pad {variant}">
  {@render children()}
</section>

<style>
  .hero {
    border-radius: var(--r-xl);
    padding: 28px;
  }
</style>
