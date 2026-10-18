import { describe, expect, it } from 'vitest'
import { filterVersions, formatReleaseDate, pickDefault, showFlags } from './versionPicker'
import type { VersionEntry } from './types'

const list: VersionEntry[] = [
  { id: '1.21', type: 'release', releaseTime: '2024-06-13T10:00:00+00:00' },
  { id: '24w21a', type: 'snapshot', releaseTime: '2024-05-22T10:00:00+00:00' },
  { id: 'b1.7.3', type: 'old_beta', releaseTime: '2011-07-08T00:00:00+00:00' },
  { id: 'a1.2.6', type: 'old_alpha', releaseTime: '2010-12-03T00:00:00+00:00' },
]

describe('selector de version', () => {
  it('cada nivel suma a los anteriores', () => {
    expect(showFlags('release')).toEqual({ release: true, snapshot: false, old_beta: false, old_alpha: false })
    expect(showFlags('snapshot').snapshot).toBe(true)
    expect(showFlags('old_beta')).toEqual({ release: true, snapshot: true, old_beta: true, old_alpha: false })
    expect(showFlags('old_alpha').old_alpha).toBe(true)
  })

  it('filtra por tipo', () => {
    expect(filterVersions(list, showFlags('release'), '').map((v) => v.id)).toEqual(['1.21'])
    expect(filterVersions(list, showFlags('old_alpha'), '')).toHaveLength(4)
  })

  it('la version elegida sigue en la lista aunque su tipo este oculto', () => {
    expect(filterVersions(list, showFlags('release'), '24w21a').map((v) => v.id)).toEqual(['24w21a', '1.21'])
  })

  it('una version desconocida se añade como estable', () => {
    const out = filterVersions(list, showFlags('release'), '1.99')
    expect(out[0]).toMatchObject({ id: '1.99', type: 'release' })
  })

  it('al cambiar de tipo de instancia pasa a la primera version disponible', () => {
    expect(pickDefault(list, showFlags('release'), '')).toBe('1.21')
    expect(pickDefault(list, showFlags('release'), '1.8.9')).toBe('1.21')
  })

  it('no cambia una version valida ni con lista vacia', () => {
    expect(pickDefault(list, showFlags('release'), 'b1.7.3')).toBeNull()
    expect(pickDefault([], showFlags('release'), '1.21')).toBeNull()
  })

  it('formatea la fecha o devuelve vacio', () => {
    expect(formatReleaseDate('2024-06-13T10:00:00+00:00')).toBe('13/06/2024')
    expect(formatReleaseDate('')).toBe('')
    expect(formatReleaseDate('no es fecha')).toBe('')
  })
})
