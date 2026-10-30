import { expect, it } from 'vitest'
import { categoryLabel, environmentLabel, formatDownloads, sinceLabel } from './modCatalog'

it('una categoria desconocida se muestra tal cual en vez de romper', () => {
  expect(categoryLabel('optimization')).toBe('Optimización')
  expect(categoryLabel('nueva-del-proveedor')).toBe('nueva-del-proveedor')
})

it('resume el entorno segun lo que declara el proveedor', () => {
  expect(environmentLabel({ clientSide: 'required', serverSide: 'unsupported' })).toBe('Solo cliente')
  expect(environmentLabel({ clientSide: 'optional', serverSide: 'required' })).toBe('Cliente y servidor')
  expect(environmentLabel({ clientSide: 'unsupported', serverSide: 'required' })).toBe('Solo servidor')
  expect(environmentLabel({ clientSide: null, serverSide: null })).toBeNull()
})

it('la fecha se dice en palabras', () => {
  const now = Date.parse('2026-10-30T00:00:00Z')
  expect(sinceLabel('2026-10-30T00:00:00Z', now)).toBe('hoy')
  expect(sinceLabel('2026-10-29T00:00:00Z', now)).toBe('ayer')
  expect(sinceLabel('2026-10-20T00:00:00Z', now)).toBe('hace 10 días')
  expect(sinceLabel('2026-08-20T00:00:00Z', now)).toBe('hace 2 meses')
  expect(sinceLabel('2024-10-01T00:00:00Z', now)).toBe('hace 2 años')
  expect(sinceLabel(null, now)).toBeNull()
  expect(sinceLabel('no es fecha', now)).toBeNull()
})

it('abrevia las descargas', () => {
  expect(formatDownloads(950)).toBe('950')
  expect(formatDownloads(12_345)).toBe('12 mil')
  expect(formatDownloads(2_500_000)).toBe('2.5 M')
  expect(formatDownloads(50_000_000)).toBe('50 M')
})
