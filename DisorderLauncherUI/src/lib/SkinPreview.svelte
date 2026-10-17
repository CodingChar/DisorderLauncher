<script lang="ts">
  import type { Skin } from './types'

  let { skin }: { skin: Skin } = $props()
</script>

<div
  class="skin-person"
  style="--sh:{skin.head};--sb:{skin.body};--sa:{skin.arm};--sl:{skin.leg}"
>
  <div class="skin-head"><i></i></div>
  <div class="skin-body"></div>
  <div class="skin-arm left"></div>
  <div class="skin-arm right"></div>
  <div class="skin-leg left"></div>
  <div class="skin-leg right"></div>
</div>