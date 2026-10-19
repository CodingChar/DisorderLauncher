const KEY = 'disorderlauncher.tour.v1'
let dismissed = false
export function needsTour(): boolean {
  if (dismissed) return false
  try { return localStorage.getItem(KEY) !== 'done' } catch { return true }
}
export function finishTour() {
  dismissed = true
  try { localStorage.setItem(KEY, 'done') } catch { /* Session fallback when storage is unavailable. */ }
}
export const TOUR = [
  ['Mis instancias', 'Crea una instancia, elige la versión de Minecraft y pulsa Jugar.'],
  ['Explorar', 'Busca modpacks para descubrir nuevas formas de jugar.'],
  ['Herramientas', 'Consulta diagnósticos y herramientas del launcher.'],
  ['Perfiles', 'Crea o cambia el perfil que usarás para jugar.'],
  ['Tu skin', 'Mira tu personaje y abre Skins debajo para crear, importar o usar una skin.'],
  ['Ajustes', 'Configura las descargas y Java. Puedes repetir este paseo desde Ayuda.'],
] as const
