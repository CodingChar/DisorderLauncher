import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Tema inicial segun el sistema (evita un parpadeo); `settings.load()` aplica la preferencia guardada.
document.documentElement.dataset.theme = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
