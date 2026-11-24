// Checking for, downloading and installing new versions from GitHub.
//
// Updates are signed in the release workflow and checked against the public
// key built into the app, so only builds from this project install. The
// library, watched folders and settings live in the app data folder, which an
// update never touches, and the app backs them up the first time a new
// version starts (see src-tauri/src/upgrade.rs).

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { api, type AppInfo } from './api';
import { flushSettings } from './settings-sync';

export const RELEASES_URL = 'https://github.com/koee-works/fontman/releases';
const AUTO_KEY = 'update-check';

type Status = 'idle' | 'checking' | 'current' | 'available' | 'downloading' | 'installing' | 'error';

class Updates {
  info = $state<AppInfo | null>(null);
  status = $state<Status>('idle');
  /** The newer version, once found. */
  available = $state<{ version: string; notes: string; date: string | null } | null>(null);
  received = $state(0);
  total = $state<number | null>(null);
  error = $state<string | null>(null);
  checkedAt = $state<number | null>(null);
  auto = $state(true);
  private update: Update | null = null;

  constructor() {
    try {
      this.auto = localStorage.getItem(AUTO_KEY) !== 'off';
    } catch {
      /* default on */
    }
  }

  setAuto(on: boolean) {
    this.auto = on;
    try {
      localStorage.setItem(AUTO_KEY, on ? 'on' : 'off');
    } catch {
      /* not persisted */
    }
  }

  /** At startup: learn the version, and check if allowed. Never throws. */
  async init() {
    try {
      this.info = await api.appInfo();
    } catch {
      return; // no backend
    }
    if (this.auto && this.info.canSelfUpdate) await this.check(true);
  }

  /**
   * Ask GitHub for a newer version. Quiet checks (at startup) don't report
   * being offline: that's not worth a message on every launch.
   */
  async check(quiet = false) {
    if (this.status === 'checking' || this.status === 'downloading' || this.status === 'installing') return;
    this.status = 'checking';
    this.error = null;
    try {
      const u = await check();
      this.checkedAt = Date.now();
      this.update = u;
      if (u) {
        this.available = { version: u.version, notes: u.body ?? '', date: u.date ?? null };
        this.status = 'available';
      } else {
        this.available = null;
        this.status = 'current';
      }
    } catch (e) {
      this.status = quiet ? 'idle' : 'error';
      this.error = quiet ? null : friendly(e);
    }
  }

  /** Download, install and restart. Settings are written out first. */
  async install() {
    if (!this.update) return;
    this.status = 'downloading';
    this.received = 0;
    this.total = null;
    this.error = null;
    try {
      await flushSettings();
      await this.update.downloadAndInstall((ev) => {
        if (ev.event === 'Started') this.total = ev.data.contentLength ?? null;
        else if (ev.event === 'Progress') this.received += ev.data.chunkLength;
        else if (ev.event === 'Finished') this.status = 'installing';
      });
      // Windows closes the app to run the installer and never gets here;
      // macOS and Linux replace it in place and restart.
      await relaunch();
    } catch (e) {
      this.status = 'error';
      this.error = friendly(e);
    }
  }
}

function friendly(e: unknown): string {
  const s = String(e);
  if (/signature/i.test(s)) return `The update's signature didn't match, so it wasn't installed. (${s})`;
  if (/network|connect|dns|timed out|offline|request/i.test(s)) return `Couldn't reach GitHub to check for updates. (${s})`;
  return s;
}

export const updates = new Updates();
