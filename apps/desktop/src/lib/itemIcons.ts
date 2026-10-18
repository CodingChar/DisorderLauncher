/**
 * Iconos de objetos para elegir como logo de una instancia. Son dibujos propios en pixel art
 * (rejilla de 16x16) compuestos con primitivas: nada de recursos externos ni imagenes pesadas.
 * El `id` es lo que se guarda en la instancia (`icon`).
 */

type Color = string | null
type Grid = Color[] // 16 x 16, fila a fila

const N = 16

function blank(): Grid {
  return new Array<Color>(N * N).fill(null)
}

function put(g: Grid, x: number, y: number, c: Color) {
  if (x >= 0 && x < N && y >= 0 && y < N) g[y * N + x] = c
}

function line(g: Grid, x0: number, y0: number, x1: number, y1: number, c: Color) {
  const dx = Math.abs(x1 - x0)
  const dy = -Math.abs(y1 - y0)
  const sx = x0 < x1 ? 1 : -1
  const sy = y0 < y1 ? 1 : -1
  let err = dx + dy
  for (;;) {
    put(g, x0, y0, c)
    if (x0 === x1 && y0 === y1) break
    const e2 = 2 * err
    if (e2 >= dy) {
      err += dy
      x0 += sx
    }
    if (e2 <= dx) {
      err += dx
      y0 += sy
    }
  }
}

function rect(g: Grid, x: number, y: number, w: number, h: number, c: Color) {
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) put(g, x + i, y + j, c)
}

function disc(g: Grid, cx: number, cy: number, r: number, c: Color) {
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) if ((x - cx) ** 2 + (y - cy) ** 2 <= r * r) put(g, x, y, c)
}

/** Linea gruesa en diagonal: dibuja la linea y su vecina para dar 2 px de ancho. */
function thick(g: Grid, x0: number, y0: number, x1: number, y1: number, c: Color) {
  line(g, x0, y0, x1, y1, c)
  line(g, x0 + 1, y0, x1 + 1, y1, c)
}

const BROWN = '#8b5a2b'
const BROWN_DARK = '#5c3a1c'

function gem(main: string, light: string, dark: string): Grid {
  const g = blank()
  const rows: [number, number, number][] = [
    [2, 5, 10],
    [3, 3, 12],
    [4, 2, 13],
    [5, 2, 13],
    [6, 3, 12],
    [7, 4, 11],
    [8, 5, 10],
    [9, 6, 9],
    [10, 7, 8],
  ]
  for (const [y, a, b] of rows) for (let x = a; x <= b; x++) put(g, x, y + 1, main)
  for (let x = 5; x <= 10; x++) put(g, x, 3, light)
  for (let y = 4; y <= 6; y++) put(g, 4, y, light)
  put(g, 5, 4, light)
  put(g, 6, 5, light)
  for (let y = 5; y <= 7; y++) put(g, 12, y, dark)
  for (let x = 8; x <= 11; x++) put(g, x, 8, dark)
  put(g, 8, 10, dark)
  put(g, 9, 9, dark)
  return g
}

function sword(blade: string, light: string, dark: string): Grid {
  const g = blank()
  thick(g, 5, 10, 12, 3, blade)
  line(g, 6, 9, 13, 2, light)
  line(g, 13, 3, 13, 2, dark)
  line(g, 7, 10, 12, 5, dark)
  line(g, 3, 8, 8, 13, BROWN_DARK) // guarda
  line(g, 4, 8, 8, 12, BROWN)
  line(g, 4, 12, 2, 14, BROWN) // empunadura
  line(g, 5, 11, 3, 13, BROWN_DARK)
  rect(g, 1, 14, 2, 1, BROWN_DARK)
  return g
}

function pickaxe(): Grid {
  const g = blank()
  line(g, 3, 14, 11, 6, BROWN)
  line(g, 4, 14, 12, 6, BROWN_DARK)
  const head: [number, number][] = [
    [4, 4], [5, 3], [6, 2], [7, 2], [8, 2], [9, 2], [10, 3], [11, 4], [12, 5], [13, 6], [13, 7], [14, 8],
    [5, 5], [6, 4], [7, 3], [8, 3], [9, 3], [10, 4], [11, 5], [12, 6], [12, 7],
  ]
  for (const [x, y] of head) put(g, x, y, '#cfd4d8')
  for (const [x, y] of [[4, 4], [5, 3], [14, 8], [13, 7]]) put(g, x, y, '#8d949a')
  for (const [x, y] of [[7, 2], [8, 2], [9, 2]]) put(g, x, y, '#f1f4f6')
  return g
}

function apple(): Grid {
  const g = blank()
  disc(g, 8, 9, 5, '#c9302c')
  disc(g, 8, 9, 4.2, '#e04a3f')
  for (const [x, y] of [[5, 7], [5, 8], [6, 6]]) put(g, x, y, '#f6a79c')
  for (const [x, y] of [[11, 11], [10, 12], [9, 13]]) put(g, x, y, '#a02420')
  line(g, 8, 4, 8, 2, BROWN)
  line(g, 9, 3, 11, 2, '#58a83c')
  put(g, 10, 3, '#3f7f2a')
  return g
}

function bow(): Grid {
  const g = blank()
  for (const [x, y] of [[10, 1], [8, 2], [6, 3], [5, 4], [4, 6], [4, 7], [4, 8], [4, 9], [5, 11], [6, 12], [8, 13], [10, 14]]) {
    put(g, x, y, BROWN)
    put(g, x + 1, y, BROWN_DARK)
  }
  line(g, 11, 1, 11, 14, '#e9e4d2')
  line(g, 5, 8, 14, 8, '#b08a5a') // flecha
  line(g, 13, 7, 14, 8, '#cfd4d8')
  line(g, 13, 9, 14, 8, '#cfd4d8')
  return g
}

function compass(): Grid {
  const g = blank()
  disc(g, 8, 8, 6.5, '#8d949a')
  disc(g, 8, 8, 5.2, '#e9e4d2')
  disc(g, 8, 8, 4.4, '#d7cfb8')
  line(g, 8, 4, 8, 8, '#d33f3a')
  line(g, 7, 5, 8, 8, '#d33f3a')
  line(g, 8, 8, 8, 12, '#5c5c5c')
  put(g, 8, 8, '#2b2b2b')
  put(g, 8, 1, '#8d949a')
  return g
}

function book(): Grid {
  const g = blank()
  rect(g, 3, 2, 10, 12, '#5b3a8c')
  rect(g, 4, 3, 8, 10, '#7a56b3')
  rect(g, 3, 2, 2, 12, '#3f2766')
  rect(g, 6, 5, 5, 1, '#e9d9a6')
  rect(g, 6, 7, 5, 1, '#e9d9a6')
  rect(g, 6, 9, 3, 1, '#e9d9a6')
  rect(g, 4, 13, 9, 1, '#e9e4d2')
  return g
}

function potion(): Grid {
  const g = blank()
  rect(g, 7, 1, 2, 2, BROWN) // corcho
  rect(g, 6, 3, 4, 3, '#cfe3ea')
  disc(g, 8, 10, 4.8, '#cfe3ea')
  disc(g, 8, 10, 3.9, '#d63c5e')
  rect(g, 7, 3, 2, 2, '#e9f4f8')
  for (const [x, y] of [[6, 9], [6, 10], [7, 8]]) put(g, x, y, '#f28aa3')
  for (const [x, y] of [[10, 12], [9, 13]]) put(g, x, y, '#a82442')
  return g
}

function heart(): Grid {
  const g = blank()
  const shape = [
    '..XXX...XXX.....',
    '.XXXXX.XXXXX....',
    'XXXXXXXXXXXXX...',
    'XXXXXXXXXXXXX...',
    'XXXXXXXXXXXXX...',
    '.XXXXXXXXXXX....',
    '..XXXXXXXXX.....',
    '...XXXXXXX......',
    '....XXXXX.......',
    '.....XXX........',
    '......X.........',
  ]
  shape.forEach((row, j) => [...row].forEach((ch, i) => ch === 'X' && put(g, i + 1, j + 2, '#d62d3a')))
  for (const [x, y] of [[3, 4], [4, 3], [3, 5], [8, 3], [9, 4]]) put(g, x, y, '#f58a93')
  for (const [x, y] of [[11, 7], [10, 9], [9, 10], [8, 11], [7, 12]]) put(g, x, y, '#9d1b26')
  return g
}

function chest(): Grid {
  const g = blank()
  rect(g, 2, 4, 12, 9, BROWN)
  rect(g, 2, 4, 12, 4, '#a06a34')
  rect(g, 2, 8, 12, 1, BROWN_DARK)
  rect(g, 2, 4, 12, 1, BROWN_DARK)
  rect(g, 2, 12, 12, 1, BROWN_DARK)
  rect(g, 2, 4, 1, 9, BROWN_DARK)
  rect(g, 13, 4, 1, 9, BROWN_DARK)
  rect(g, 7, 7, 2, 3, '#d8d8d8')
  rect(g, 7, 7, 2, 1, '#8d949a')
  return g
}

function torch(): Grid {
  const g = blank()
  rect(g, 7, 6, 2, 9, BROWN)
  rect(g, 8, 6, 1, 9, BROWN_DARK)
  rect(g, 6, 3, 4, 3, '#f2a02d')
  rect(g, 7, 2, 2, 1, '#f2a02d')
  rect(g, 7, 4, 2, 2, '#ffe066')
  put(g, 8, 1, '#f26b2d')
  put(g, 7, 3, '#f26b2d')
  return g
}

export interface ItemIcon {
  id: string
  label: string
  pixels: Grid
}

export const ITEM_ICONS: ItemIcon[] = [
  { id: 'diamond-sword', label: 'Espada de diamante', pixels: sword('#38c9c4', '#c8fffa', '#1c8d8a') },
  { id: 'iron-pickaxe', label: 'Pico de hierro', pixels: pickaxe() },
  { id: 'apple', label: 'Manzana', pixels: apple() },
  { id: 'diamond', label: 'Diamante', pixels: gem('#45d4cd', '#c8fffa', '#1c8d8a') },
  { id: 'emerald', label: 'Esmeralda', pixels: gem('#2fc765', '#b5f5cc', '#18803f') },
  { id: 'bow', label: 'Arco', pixels: bow() },
  { id: 'compass', label: 'Brújula', pixels: compass() },
  { id: 'book', label: 'Libro', pixels: book() },
  { id: 'potion', label: 'Poción', pixels: potion() },
  { id: 'heart', label: 'Corazón', pixels: heart() },
  { id: 'chest', label: 'Cofre', pixels: chest() },
  { id: 'torch', label: 'Antorcha', pixels: torch() },
]

export function findItemIcon(id: string | null | undefined): ItemIcon | undefined {
  return id ? ITEM_ICONS.find((i) => i.id === id) : undefined
}
