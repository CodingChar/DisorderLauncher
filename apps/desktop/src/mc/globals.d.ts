export {}

declare global {
  interface Window {
    /** Solo DEV: fuerza el tamano virtual del escenario en px de GUI. */
    __forceGui?: { w: number; h: number }
    /** Solo DEV: auditor de overflow del modo Minecraft (src/mc/audit.ts). */
    __mcAudit?: () => unknown[]
    /** Solo DEV: el store de navegacion. */
    __mcNav?: { view: string; go: (view: never) => void }
  }
}
