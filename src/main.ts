import { mount } from 'svelte';
import App from './App.svelte';
import './app.css';
// Timing tokens on <html> before the first paint.
import './lib/motion.svelte';

export default mount(App, { target: document.getElementById('app')! });
