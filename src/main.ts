import { mount } from 'svelte';
import './app.css';
import { restoreSettings } from './lib/settings-sync';

// Saved settings go back into storage before any module reads them, so the
// app is imported only after. Motion first: its timing tokens are set on
// <html> before the first paint.
const app = restoreSettings().then(async () => {
  await import('./lib/motion.svelte');
  const { default: App } = await import('./App.svelte');
  return mount(App, { target: document.getElementById('app')! });
});

export default app;
