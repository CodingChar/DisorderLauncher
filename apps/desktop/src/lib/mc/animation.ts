/** Animaciones decorativas: sin trabajo en segundo plano y con limite de refresco. */
export function animateVisible(draw: (now: number, still: boolean) => void, fps = 60): () => void {
  const motion = matchMedia('(prefers-reduced-motion: reduce)')
  const interval = 1000 / fps
  let raf = 0
  let last = -Infinity
  const frame = (now: number) => {
    if (now - last >= interval - 0.5) {
      last = now
      draw(now, false)
    }
    raf = requestAnimationFrame(frame)
  }
  const sync = () => {
    cancelAnimationFrame(raf)
    last = -Infinity
    if (document.hidden) return
    if (motion.matches) draw(0, true)
    else raf = requestAnimationFrame(frame)
  }
  sync()
  document.addEventListener('visibilitychange', sync)
  motion.addEventListener('change', sync)
  return () => {
    cancelAnimationFrame(raf)
    document.removeEventListener('visibilitychange', sync)
    motion.removeEventListener('change', sync)
  }
}
