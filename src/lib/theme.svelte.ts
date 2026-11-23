// Appearance: a mode (follow the system, or force light or dark) and, as in
// Zed, a theme for each of light and dark. "Font Manager" is the app's own
// palette in app.css; the others are Zed themes, bundled or imported, mapped
// onto the same tokens by themes/zed.ts.
//
// The mode is applied as data-theme on <html>; a Zed theme as inline custom
// properties over it. Everything is stored in localStorage, guarded, with the
// two chosen themes cached so they apply at startup before anything loads.

import { api, type UserTheme } from './api';
import builtin from './themes/builtin.json';
import { themeTokens, type ZedFamily, type ZedTheme } from './themes/zed';

export type ThemePreference = 'system' | 'light' | 'dark';
export type Appearance = 'light' | 'dark';

/** The app's own palette. */
export const DEFAULT_THEME = 'default';

export interface ThemeOption {
  id: string;
  label: string;
  /** Imported themes can be removed; this is their file. */
  file?: string;
}

const KEY = 'appearance';
const LIGHT_KEY = 'theme-light';
const DARK_KEY = 'theme-dark';
const CACHE_KEY = 'theme-cache';

function get(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}
function put(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* not persisted; still applied for this session */
  }
}

function readMode(): ThemePreference {
  const v = get(KEY);
  return v === 'light' || v === 'dark' || v === 'system' ? v : 'system';
}

const builtinId = (t: ZedTheme) => `zed:${t.name}`;
const userId = (file: string, t: ZedTheme) => `user:${file}:${t.name}`;

class Theme {
  preference = $state<ThemePreference>(readMode());
  light = $state(get(LIGHT_KEY) ?? DEFAULT_THEME);
  dark = $state(get(DARK_KEY) ?? DEFAULT_THEME);
  imported = $state.raw<UserTheme[]>([]);
  /** Colours adjusted for contrast in the theme showing now. */
  adjusted = $state.raw<string[]>([]);
  private systemDark = $state(false);
  private cache: Record<string, ZedTheme> = (() => {
    try {
      return JSON.parse(get(CACHE_KEY) ?? '{}');
    } catch {
      return {};
    }
  })();

  constructor() {
    const mq = typeof matchMedia === 'function' ? matchMedia('(prefers-color-scheme: dark)') : null;
    this.systemDark = mq?.matches ?? false;
    mq?.addEventListener('change', (e) => {
      this.systemDark = e.matches;
      this.apply();
    });
    this.apply();
  }

  /** Light or dark, as shown now. */
  get appearance(): Appearance {
    return this.preference === 'system' ? (this.systemDark ? 'dark' : 'light') : this.preference;
  }

  /** Themes to choose from for one appearance. */
  options(appearance: Appearance): ThemeOption[] {
    const out: ThemeOption[] = [{ id: DEFAULT_THEME, label: 'Font Manager' }];
    for (const fam of builtin as ZedFamily[])
      for (const t of fam.themes) if (t.appearance === appearance) out.push({ id: builtinId(t), label: t.name });
    for (const u of this.imported)
      for (const t of u.family.themes)
        if (t.appearance === appearance) out.push({ id: userId(u.file, t), label: t.name, file: u.file });
    return out;
  }

  private find(id: string): ZedTheme | null {
    if (id === DEFAULT_THEME) return null;
    for (const fam of builtin as ZedFamily[]) for (const t of fam.themes) if (builtinId(t) === id) return t;
    for (const u of this.imported) for (const t of u.family.themes) if (userId(u.file, t) === id) return t;
    return this.cache[id] ?? null;
  }

  /** Custom properties the current theme set on <html>. */
  private applied: string[] = [];

  private apply() {
    const root = document.documentElement;
    const appearance = this.appearance;
    const t = this.find(appearance === 'dark' ? this.dark : this.light);
    // Clear the previous theme's colours, and only those: other settings
    // (motion's timings) keep their properties on <html> too.
    for (const name of this.applied) root.style.removeProperty(name);
    this.applied = [];
    if (t) {
      // A Zed theme: its appearance's base, with its colours on top.
      root.dataset.theme = appearance;
      const { vars, adjusted } = themeTokens(t);
      for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
      this.applied = Object.keys(vars);
      this.adjusted = adjusted;
    } else {
      if (this.preference === 'system') delete root.dataset.theme;
      else root.dataset.theme = this.preference;
      this.adjusted = [];
    }
  }

  private remember() {
    const keep: Record<string, ZedTheme> = {};
    for (const id of [this.light, this.dark]) {
      const t = this.find(id);
      if (t && id.startsWith('user:')) keep[id] = t;
    }
    this.cache = keep;
    put(CACHE_KEY, JSON.stringify(keep));
  }

  set(pref: ThemePreference) {
    this.preference = pref;
    put(KEY, pref);
    this.apply();
  }

  /** Choose the theme used in light or dark appearance. */
  choose(appearance: Appearance, id: string) {
    if (appearance === 'dark') {
      this.dark = id;
      put(DARK_KEY, id);
    } else {
      this.light = id;
      put(LIGHT_KEY, id);
    }
    this.remember();
    this.apply();
  }

  async loadImported() {
    this.imported = await api.listUserThemes();
    // A chosen theme whose file is gone falls back to the app's own.
    for (const a of ['light', 'dark'] as Appearance[]) {
      const id = a === 'dark' ? this.dark : this.light;
      if (!this.find(id)) this.choose(a, DEFAULT_THEME);
    }
    this.remember();
    this.apply();
  }

  /** Import a Zed theme file. Returns its family, or null if cancelled. */
  async importFile(): Promise<UserTheme | null> {
    const u = await api.importZedTheme();
    if (u) {
      this.imported = [...this.imported.filter((x) => x.file !== u.file), u];
      this.apply();
    }
    return u;
  }

  async remove(file: string) {
    await api.removeUserTheme(file);
    this.imported = this.imported.filter((u) => u.file !== file);
    for (const a of ['light', 'dark'] as Appearance[]) {
      const id = a === 'dark' ? this.dark : this.light;
      if (id.startsWith(`user:${file}:`)) this.choose(a, DEFAULT_THEME);
    }
  }
}

export const theme = new Theme();
