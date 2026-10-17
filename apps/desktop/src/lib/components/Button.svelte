<script lang="ts">
  import type { Snippet } from 'svelte'

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost'
    type?: 'button' | 'submit'
    disabled?: boolean
    large?: boolean
    onclick?: (event: MouseEvent) => void
    children: Snippet
  }

  let { variant = 'secondary', type = 'button', disabled = false, large = false, onclick, children }: Props = $props()
</script>

<!-- Los estilos (`.primary`, `.secondary`, `.ghost`) son globales: ver app.css. -->
<button class={variant === 'ghost' ? 'secondary ghost' : variant} class:large {type} {disabled} {onclick}>
  {@render children()}
</button>
