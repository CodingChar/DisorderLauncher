/** Capa de un icono de fila: una imagen entera o un recorte de ella (por ejemplo la cara de una skin). */
export interface RowIconLayer {
  src: string
  /** Recorte `[x, y, ancho, alto]` en px de la imagen original. */
  crop?: [number, number, number, number]
  /** Ancho de la imagen original (necesario con `crop`). */
  imageWidth?: number
}

export interface RowLine {
  text: string
  color?: string
}

/** Lo que `McList` necesita para dibujar una fila. */
export interface RowSpec {
  /** Una o varias capas (la cara y el sombrero de una skin, por ejemplo). */
  icon?: RowIconLayer | RowIconLayer[]
  /** Hasta tres lineas; cada una se acorta con «...» si no cabe. */
  lines: RowLine[]
  disabled?: boolean
}

export interface GridIcon {
  id: string
  label: string
  src: string
}
