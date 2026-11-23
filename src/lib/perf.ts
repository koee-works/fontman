// Timing marks for measuring the UI against large libraries.
// Compiled out unless built with VITE_PERF=1. Marks render into a fixed
// overlay, so they can be read from a screenshot of the running app.

const enabled = import.meta.env.VITE_PERF === '1';
const marks: string[] = [];

let live = '';

let overlay: HTMLElement | null = null;

function render() {
  if (!overlay) {
    overlay = document.createElement('div');
    overlay.style.cssText =
      'position:fixed;left:8px;bottom:8px;z-index:99;padding:4px 8px;' +
      'background:#000;color:#0f0;font:14px monospace;white-space:pre';
    document.body.append(overlay);
  }
  overlay.textContent = marks.join('  ') + (live ? '\n' + live : '');
}

export function mark(name: string, ms: number) {
  if (!enabled) return;
  marks.push(`${name}=${Math.round(ms)}`);
  render();
}

/** A value that is overwritten rather than appended, e.g. running counters. */
export function liveStatus(text: string) {
  if (!enabled) return;
  live = text;
  render();
}

/** Resolves after the browser has painted the current DOM. */
export function painted(): Promise<void> {
  return new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(() => r())));
}

export const perfEnabled = enabled;
