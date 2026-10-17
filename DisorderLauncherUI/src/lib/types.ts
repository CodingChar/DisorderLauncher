export type Instance = {
  name: string
  version: string
  loader: string
  mods: number
  type: string
  time: string
  minRam: number
  maxRam: number
  compatibility: string
  java: string
  args: string
  width: number
  height: number
  fullscreen: boolean
}

export type Session = {
  instance: Instance
  profile: string
  started: string
  logs: string[]
}

export type Skin = {
  id: string
  name: string
  head: string
  body: string
  arm: string
  leg: string
  equipped: boolean
}

export type Profile = {
  id: string
  name: string
  created: string
  skins: Skin[]
}

export const defaults: Instance = {
  name: "",
  version: "1.21.4",
  loader: "Fabric",
  mods: 0,
  type: "grass",
  time: "Sin jugar todavía",
  minRam: 1,
  maxRam: 4,
  compatibility: "Automático",
  java: "Automático",
  args: "-XX:+UseG1GC",
  width: 1280,
  height: 720,
  fullscreen: false,
}

/** Iconos disponibles para identificar una instancia. */
export const iconItems: [string, string][] = [
  ["grass", "Bloque de césped"],
  ["stone", "Piedra"],
  ["sand", "Arena"],
  ["diamond", "Diamante"],
  ["sword", "Espada"],
  ["apple", "Manzana"],
]

export const blockTypes = ["grass", "stone", "sand"]

export const mcVersions = ["1.21.4", "1.21.1", "1.20.1"]

export const loaders = ["Vanilla", "Fabric", "Forge", "NeoForge", "Quilt"]

export const javaRuntimes = ["Automático", "Java 21", "Java 17", "Java 8"]

export const compatibilityModes = [
  "Automático",
  "Estándar",
  "Gráficos heredados",
  "Modo seguro (sin mods)",
]