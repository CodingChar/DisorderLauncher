import { mount } from 'svelte'
import './index.css'
import App from './App.svelte'

const target = document.getElementById('root')
if (!target) throw new Error('No se encontró el elemento #root en index.html')

export default mount(App, { target })