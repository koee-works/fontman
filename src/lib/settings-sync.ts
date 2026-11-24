// The interface's settings (theme, motion, layout, filters…) are read and
// written through localStorage everywhere in the app. That storage belongs to
// the webview, which an uninstall or a webview reset can clear, so it is
// mirrored to ui-settings.json in the app data folder, next to the library:
// the copy that installers and the updater never touch, and that is backed up
// before each update.
//
// At startup the file wins: its values are put back into localStorage before
// any module reads them. After that, every change is written back to the
// file, a moment later. Where there's no backend (tests in a browser) this
// does nothing and localStorage works as usual.

import { invoke } from '@tauri-apps/api/core';

const SAVE_DELAY_MS = 400;
let timer: ReturnType<typeof setTimeout> | null = null;
let active = false;

function snapshot(): Record<string, string> {
  const out: Record<string, string> = {};
  for (let i = 0; i < localStorage.length; i++) {
    const k = localStorage.key(i);
    if (k !== null) out[k] = localStorage.getItem(k) ?? '';
  }
  return out;
}

async function save() {
  timer = null;
  try {
    await invoke('save_ui_settings', { settings: snapshot() });
  } catch (e) {
    console.warn('Settings could not be saved to the app data folder:', e);
  }
}

function schedule() {
  if (!active) return;
  if (timer) clearTimeout(timer);
  timer = setTimeout(save, SAVE_DELAY_MS);
}

/** Write any pending change now: before an update installs and restarts. */
export async function flushSettings() {
  if (!active) return;
  if (timer) clearTimeout(timer);
  await save();
}

/** Put saved settings back, then keep the file up to date. Call before the app loads. */
export async function restoreSettings() {
  let saved: Record<string, unknown> | null;
  try {
    saved = await invoke<Record<string, unknown> | null>('load_ui_settings');
  } catch {
    return; // no backend
  }
  try {
    if (saved) {
      for (const [k, v] of Object.entries(saved)) {
        if (typeof v === 'string') localStorage.setItem(k, v);
      }
    }
  } catch {
    return; // storage unavailable: nothing to mirror
  }

  // Every write goes through these, whichever module makes it.
  const proto = Storage.prototype;
  const setItem = proto.setItem;
  const removeItem = proto.removeItem;
  const clear = proto.clear;
  proto.setItem = function (this: Storage, key: string, value: string) {
    setItem.call(this, key, value);
    if (this === localStorage) schedule();
  };
  proto.removeItem = function (this: Storage, key: string) {
    removeItem.call(this, key);
    if (this === localStorage) schedule();
  };
  proto.clear = function (this: Storage) {
    clear.call(this);
    if (this === localStorage) schedule();
  };
  active = true;

  // First launch with this file: seed it from what the webview already has,
  // so settings from older versions carry over.
  if (!saved) await save();
  addEventListener('pagehide', () => void flushSettings());
}
