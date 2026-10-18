export {}

declare global {
  interface Window {
    /** Solo DEV: fuerza el tamano virtual del escenario en px de GUI. */
    __forceGui?: { w: number; h: number }
    /** Solo DEV: auditor de overflow del modo Minecraft (src/mc/audit.ts). */
    __mcAudit?: () => unknown[]
    /** Solo DEV: recorre todas las pantallas y devuelve los problemas del auditor. */
    __mcMatrix?: (options?: { seed?: boolean; extra?: number }) => Promise<string[]>
    /** Solo DEV: el store de navegacion. */
    __mcNav?: { view: string; go: (view: never) => void }
  }
}
