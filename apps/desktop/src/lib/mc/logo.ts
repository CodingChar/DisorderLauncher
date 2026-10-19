// Logo «DISORDERLAUNCHER» en letras de bloque de piedra (como el de Minecraft 1.7.10), dibujado en un lienzo.
const LOGO_TEXT = 'DISORDERLAUNCHER';
const GLYPHS: Record<string, string[]> = {
  A: ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
  B: ["11110", "10001", "10001", "11110", "10001", "10001", "11110"],
  C: ["01110", "10001", "10000", "10000", "10000", "10001", "01110"],
  D: ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
  E: ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
  F: ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
  G: ["01110", "10001", "10000", "10111", "10001", "10001", "01111"],
  H: ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
  I: ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
  J: ["00111", "00010", "00010", "00010", "00010", "10010", "01100"],
  K: ["10001", "10010", "10100", "11000", "10100", "10010", "10001"],
  L: ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
  M: ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
  N: ["10001", "11001", "10101", "10101", "10011", "10001", "10001"],
  O: ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
  P: ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
  Q: ["01110", "10001", "10001", "10001", "10101", "10010", "01101"],
  R: ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
  S: ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
  T: ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
  U: ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
  V: ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
  W: ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
  X: ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
  Y: ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
  Z: ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
};

function mulberry32(seed: number) {
  return () => {
    seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function buildCells(text: string) {
  const cells = new Set<string>();
  let col = 0;
  for (const ch of text.toUpperCase()) {
    const glyph = GLYPHS[ch];
    if (glyph) {
      glyph.forEach((row, y) => {
        for (let x = 0; x < 5; x++) if (row[x] === "1") cells.add(`${col + x},${y}`);
      });
    }
    col += 6;
  }
  return { cells, cols: Math.max(0, col - 1), rows: 7 };
}

const gray = (v: number) => `rgb(${v},${v},${v})`;
const grid = buildCells(LOGO_TEXT);
const rasters = new Map<number, HTMLCanvasElement>();

export function drawLogo(logo: HTMLCanvasElement, state: { width: number; scale: number }): { width: number; height: number } {
  const { cells, cols, rows } = grid;
  const has = (x: number, y: number) => cells.has(`${x},${y}`);
  const scale = state.scale;

  // Tamaño de bloque en píxeles reales; el logo nunca supera ~92 % del ancho.
  const cellGui = Math.min(4, (state.width * 0.82) / (cols + 3));
  const c = Math.max(3, Math.floor(cellGui * scale));
  const outline = Math.max(1, Math.round(c / 8));
  const depth = Math.max(2, Math.round(c * 0.6));
  const q = Math.max(1, Math.floor(c / 4));           // grano de la piedra

  const w = cols * c + depth + outline * 2;
  const h = rows * c + depth + outline * 2;
  logo.width = w;
  logo.height = h;

  const ctx = logo.getContext("2d")!;
  ctx.imageSmoothingEnabled = false;
  const fit = Math.min(1, (state.width * 0.8) / (w / scale));
  const size = { width: (w / scale) * fit, height: (h / scale) * fit };
  const cached = rasters.get(c);
  if (cached) {
    ctx.drawImage(cached, 0, 0);
    return size;
  }
  ctx.clearRect(0, 0, w, h);
  const list = [...cells].map((k) => k.split(",").map(Number) as [number, number]);

  // Volumen: capas desde el fondo hasta la cara frontal.
  for (let d = depth; d >= 0; d--) {
    ctx.fillStyle = "#000";
    for (const [x, y] of list) {
      ctx.fillRect(outline + x * c + d - outline, outline + y * c + d - outline, c + outline * 2, c + outline * 2);
    }
    if (d > 0) {
      const side = 104 - Math.round((d / depth) * 34);
      ctx.fillStyle = gray(side);
      for (const [x, y] of list) ctx.fillRect(outline + x * c + d, outline + y * c + d, c, c);
    }
  }

  // Cara frontal: piedra con grano, bisel claro arriba-izquierda y oscuro abajo-derecha.
  const rnd = mulberry32(1710);
  for (const [x, y] of list) {
    const px = outline + x * c;
    const py = outline + y * c;
    const tone = 176 - Math.round((y / rows) * 30);
    const top = !has(x, y - 1), left = !has(x - 1, y);
    const bottom = !has(x, y + 1), right = !has(x + 1, y);

    for (let sy = 0; sy < c; sy += q) {
      for (let sx = 0; sx < c; sx += q) {
        let v = tone + Math.round((rnd() - 0.5) * 22);
        if (top && sy < q) v += 42;
        if (left && sx < q) v += 28;
        if (bottom && sy + q >= c) v -= 22;
        if (right && sx + q >= c) v -= 14;
        ctx.fillStyle = gray(Math.max(30, Math.min(236, v)));
        ctx.fillRect(px + sx, py + sy, Math.min(q, c - sx), Math.min(q, c - sy));
      }
    }
  }

  const snapshot = document.createElement('canvas');
  snapshot.width = w;
  snapshot.height = h;
  snapshot.getContext('2d')!.drawImage(logo, 0, 0);
  if (rasters.size >= 4) rasters.delete(rasters.keys().next().value!);
  rasters.set(c, snapshot);
  return size;
}

