<script lang="ts">
  import { boxParts, faceRects, type BoxPart, type FaceName } from '../skinEditor'
  import type { SkinModel } from '../types'

  interface Props {
    /** Data URL del PNG 64x64. */
    src: string
    model: SkinModel
    /** Pixeles de pantalla por pixel de skin. */
    unit?: number
  }

  let { src, model, unit = 7 }: Props = $props()

  // Centro de cada pieza (en pixeles de skin, y hacia abajo) y escala de la capa exterior.
  const OVERLAY_SCALE = 1.0625
  let yaw = $state(-28)
  let pitch = $state(-12)
  let dragging = $state(false)
  let last = { x: 0, y: 0 }

  const aw = $derived(model === 'slim' ? 3 : 4)
  const centers = $derived<Record<string, [number, number]>>({
    head: [0, 4],
    hat: [0, 4],
    body: [0, 14],
    jacket: [0, 14],
    rightArm: [-(4 + aw / 2), 14],
    rightSleeve: [-(4 + aw / 2), 14],
    leftArm: [4 + aw / 2, 14],
    leftSleeve: [4 + aw / 2, 14],
    rightLeg: [-2, 26],
    rightPants: [-2, 26],
    leftLeg: [2, 26],
    leftPants: [2, 26],
  })

  const parts = $derived(boxParts(model))
  const FACES: FaceName[] = ['front', 'back', 'right', 'left', 'top', 'bottom']

  /** Transformacion de cada cara: su frente mira hacia fuera de la pieza. */
  function faceTransform(face: FaceName, p: BoxPart): string {
    const half = (n: number) => (n / 2) * unit
    switch (face) {
      case 'front':
        return `translateZ(${half(p.d)}px)`
      case 'back':
        return `rotateY(180deg) translateZ(${half(p.d)}px)`
      case 'right':
        return `rotateY(-90deg) translateZ(${half(p.w)}px)`
      case 'left':
        return `rotateY(90deg) translateZ(${half(p.w)}px)`
      case 'top':
        return `rotateX(90deg) translateZ(${half(p.h)}px)`
      case 'bottom':
        return `rotateX(-90deg) translateZ(${half(p.h)}px)`
    }
  }

  function onDown(e: PointerEvent) {
    dragging = true
    last = { x: e.clientX, y: e.clientY }
    ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  }

  function onMove(e: PointerEvent) {
    if (!dragging) return
    yaw += (e.clientX - last.x) * 0.6
    pitch = Math.max(-80, Math.min(80, pitch - (e.clientY - last.y) * 0.6))
    last = { x: e.clientX, y: e.clientY }
  }

  const turn = (deg: number) => (yaw = deg)
</script>

<div class="viewer">
  <div
    class="stage"
    role="img"
    aria-label="Vista previa 3D de la skin. Arrastra para girarla."
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={() => (dragging = false)}
    onpointercancel={() => (dragging = false)}
    style:--tex={src ? `url(${src})` : 'none'}
    style:--u="{unit}px"
  >
    <div class="scene" style:transform="translateY(-{16 * unit}px) rotateX({pitch}deg) rotateY({yaw}deg)">
      {#each parts as part (part.name)}
        {@const c = centers[part.name]}
        {@const overlay = part.layer === 'overlay'}
        <div
          class="part"
          style:transform="translate3d({c[0] * unit}px, {c[1] * unit}px, 0) scale3d({overlay ? OVERLAY_SCALE : 1}, {overlay ? OVERLAY_SCALE : 1}, {overlay ? OVERLAY_SCALE : 1})"
        >
          {#each FACES as face (face)}
            {@const r = faceRects(part)[face]}
            <div
              class="face"
              style:width="{r.w * unit}px"
              style:height="{r.h * unit}px"
              style:margin-left="-{(r.w * unit) / 2}px"
              style:margin-top="-{(r.h * unit) / 2}px"
              style:background-size="{64 * unit}px {64 * unit}px"
              style:background-position="-{r.x * unit}px -{r.y * unit}px"
              style:transform={faceTransform(face, part)}
            ></div>
          {/each}
        </div>
      {/each}
    </div>
  </div>

  <div class="turns" role="group" aria-label="Girar la vista">
    <button type="button" onclick={() => turn(0)}>Frente</button>
    <button type="button" onclick={() => turn(90)}>Izquierda</button>
    <button type="button" onclick={() => turn(180)}>Espalda</button>
    <button type="button" onclick={() => turn(-90)}>Derecha</button>
  </div>
</div>

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: center;
  }

  .stage {
    width: 100%;
    height: 300px;
    perspective: 700px;
    perspective-origin: 50% 40%;
    display: grid;
    place-items: center;
    touch-action: none;
    cursor: grab;
    background: var(--surface-alt);
    border-radius: var(--r-md);
    overflow: hidden;
    position: relative;
  }

  .stage:active {
    cursor: grabbing;
  }

  .scene {
    position: absolute;
    left: 50%;
    top: 62%;
    width: 0;
    height: 0;
    transform-style: preserve-3d;
  }

  .part {
    position: absolute;
    transform-style: preserve-3d;
  }

  .face {
    position: absolute;
    left: 0;
    top: 0;
    background-image: var(--tex);
    background-repeat: no-repeat;
    image-rendering: pixelated;
    backface-visibility: hidden;
  }

  .turns {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .turns button {
    min-height: 40px;
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--border-strong);
    background: var(--surface-alt);
    color: var(--text-primary);
  }

  .turns button:hover {
    background: var(--surface-raised);
  }
</style>
