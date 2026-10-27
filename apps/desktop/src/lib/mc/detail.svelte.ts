/** Ventana con el texto completo de algo que en pantalla se ve acortado (avisos, filas de listas...). */
class DetailState {
  open = $state(false)
  title = $state('')
  text = $state('')

  show(title: string, text: string) {
    this.title = title
    this.text = text
    this.open = true
  }

  close() {
    this.open = false
  }
}

export const detail = new DetailState()
