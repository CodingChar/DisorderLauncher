import { mount } from 'svelte'
import './mc/stage.css'
import './mc/screens.css'
import App from './App.svelte'

// Tema inicial segun el sistema (evita un parpadeo); `settings.load()` aplica la preferencia guardada.
document.documentElement.dataset.theme = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'

if (import.meta.env.DEV) void import('./mc/audit')

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
