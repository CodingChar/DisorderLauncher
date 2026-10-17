import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'

// Tema segun el sistema; las preferencias de usuario llegan en Sprint 2.
document.documentElement.dataset.theme = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark'

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
