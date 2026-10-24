// Preferencia «Fondo en movimiento» del panorama. Por defecto gira, como en el Minecraft original, aunque Windows
// tenga las animaciones del sistema desactivadas: es un giro lento (2 grados por segundo) y se puede apagar en Ajustes.
const STORAGE_KEY = 'disorder.panoramaMoves'

export function panoramaMoves(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== 'off'
  } catch {
    return true
  }
}

export function setPanoramaMoves(moves: boolean) {
  try {
    localStorage.setItem(STORAGE_KEY, moves ? 'on' : 'off')
  } catch { /* Sin almacenamiento: vale solo esta sesion. */ }
}
