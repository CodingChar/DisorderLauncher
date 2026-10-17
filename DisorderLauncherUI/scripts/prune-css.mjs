// Elimina del CSS las reglas cuyo selector usa una clase que ya no aparece
// en el codigo Svelte. Parseo a nivel de regla (el CSS esta minificado y
// contiene varios rules por linea), respetando los bloques @media.
import { readFileSync, writeFileSync } from 'node:fs'

const path = 'src/index.css'
const css = readFileSync(path, 'utf8')

const source = ['App.svelte', 'lib/Icon.svelte', 'lib/ItemLogo.svelte', 'lib/Avatar.svelte', 'lib/SkinPreview.svelte', 'lib/SessionMonitor.svelte', 'lib/InstanceEditor.svelte', 'lib/types.ts', 'main.ts']
  .map((f) => readFileSync(`src/${f}`, 'utf8'))
  .join('\n')

// Clases que Svelte genera de forma dinamica y no aparecen literales.
const dynamic = new Set(['art-1', 'art-2'])

const allClasses = new Set()
for (const m of css.matchAll(/(?<=[},])\.([a-zA-Z][\w-]*)/g)) allClasses.add(m[1])
for (const m of css.matchAll(/(^|[\s>])\.([a-zA-Z][\w-]*)/gm)) allClasses.add(m[2])

const dead = new Set()
for (const cls of allClasses) {
  if (dynamic.has(cls)) continue
  if (!new RegExp(`(?<![\\w-])${cls}(?![\\w-])`).test(source)) dead.add(cls)
}

const usesDead = (selector) =>
  [...dead].some((cls) => new RegExp(`\\.${cls}(?![\\w-])`).test(selector))

/** Devuelve [limpieza, reglasEliminadas] para un bloque de texto. */
function sweep(block, depth = 0) {
  let out = ''
  let i = 0
  const removed = []
  while (i < block.length) {
    const braceAt = block.indexOf('{', i)
    if (braceAt === -1) {
      out += block.slice(i)
      break
    }
    const prelude = block.slice(i, braceAt)
    // Localiza el cierre respetando anidamiento (solo @media anida reglas).
    let depthCur = 1
    let j = braceAt + 1
    while (j < block.length && depthCur > 0) {
      if (block[j] === '{') depthCur++
      else if (block[j] === '}') depthCur--
      j++
    }
    const body = block.slice(braceAt + 1, j - 1)
    const isAtRule = prelude.trimStart().startsWith('@')

    if (isAtRule) {
      const [inner, innerRemoved] = sweep(body, depth + 1)
      out += prelude + '{' + inner + '}'
      removed.push(...innerRemoved)
    } else if (usesDead(prelude)) {
      removed.push(prelude.trim())
    } else {
      out += prelude + '{' + body + '}'
    }
    i = j
  }
  return [out, removed]
}

const [cleaned, removed] = sweep(css)
writeFileSync(path, cleaned, 'utf8')

console.log(`clases muertas detectadas: ${dead.size}`)
console.log(`reglas eliminadas: ${removed.length}`)
console.log(removed.map((r) => '  - ' + r).join('\n'))