import { afterEach, expect, it, vi } from 'vitest'
import { api } from '../api'
import { settings } from './settings.svelte'
import { clearThemeVars } from '../theme'
vi.mock('../theme', () => ({ clearThemeVars: vi.fn() }))
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals() })
it('ignores saved themes without reading theme catalogs and saves Minecraft defaults', async () => {
  vi.stubGlobal('document', { documentElement: { dataset: {} } })
  const old = { ...settings.value, theme: 'light' as const, themeId: 'custom', javaPath: 'java', downloadProfile: 'eco' as const }
  vi.spyOn(api, 'settingsGet').mockResolvedValue({ settings: old, dataDir: 'test' })
  const themes = vi.spyOn(api, 'themesList')
  const save = vi.spyOn(api, 'settingsUpdate').mockImplementation(async (value) => ({ settings: value, dataDir: 'test' }))
  await settings.load()
  expect(themes).not.toHaveBeenCalled()
  expect(clearThemeVars).toHaveBeenCalled()
  expect(settings.value).toMatchObject({ theme: 'dark', themeId: null, javaPath: 'java', downloadProfile: 'eco' })
  await settings.save(old)
  expect(save).toHaveBeenCalledWith({ ...old, theme: 'dark', themeId: null })
  expect(document.documentElement.dataset.theme).toBe('dark')
})
