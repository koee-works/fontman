// Style viewer state, shared so the inspector can send a face to the viewer.
// Persisted to localStorage (guarded: storage can be unavailable).

import { api, type FaceSummary, type FamilySummary, type Pairing } from '../api';
import {
  defaultSettings,
  pairingOf,
  parsePairing,
  samePairing,
  STYLES,
  type FontChoice,
  type PairingSettings,
  type Role,
  type ViewerSettings,
} from './model';

const KEY = 'style-viewer';
const PAIRING_KEY = 'style-viewer-pairing';

function loadPairingId(): number | null {
  try {
    const n = Number(localStorage.getItem(PAIRING_KEY));
    return Number.isInteger(n) && n > 0 ? n : null;
  } catch {
    return null;
  }
}

function load(): ViewerSettings {
  const d = defaultSettings();
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return d;
    const saved = JSON.parse(raw) as Partial<ViewerSettings>;
    // Merge so settings added in later versions get their defaults.
    return {
      ...d,
      ...saved,
      styles: { ...d.styles, ...(saved.styles ?? {}) },
      texts: { ...d.texts, ...(saved.texts ?? {}) },
    };
  } catch {
    return d;
  }
}

/** The face closest to a target weight, upright preferred. */
export function faceNear(faces: FaceSummary[], weight: number): FaceSummary | undefined {
  return [...faces].sort(
    (a, b) =>
      Number(a.italic) - Number(b.italic) ||
      Math.abs(a.weight - weight) - Math.abs(b.weight - weight),
  )[0];
}

/** Headings default to the face nearest bold; body text to the regular. */
export const ROLE_WEIGHT: Record<Role, number> = { heading: 700, body: 400 };

/**
 * Find a saved font choice in the library as it is now. The exact face if
 * it's still there; otherwise the same family's nearest face (a font file
 * that was updated gets a new id); otherwise nothing.
 */
export function locate(
  choice: FontChoice | null,
  role: Role,
  families: FamilySummary[],
): FontChoice | null {
  if (!choice) return null;
  for (const f of families) if (f.faces.some((x) => x.id === choice.faceId)) return choice;
  const fam = families.find((f) => f.name.toLowerCase() === choice.family.toLowerCase());
  const face = fam && faceNear(fam.faces, ROLE_WEIGHT[role]);
  return fam && face ? { family: fam.name, faceId: face.id } : null;
}

/** A pairing's settings with its fonts located in the current library. */
export function locatePairing(p: PairingSettings, families: FamilySummary[]): PairingSettings {
  return { ...p, heading: locate(p.heading, 'heading', families), body: locate(p.body, 'body', families) };
}

class ViewerState {
  s = $state<ViewerSettings>(load());
  /** Bumped when text is reset, so edited blocks remount with fresh text. */
  textVersion = $state(0);

  /** Saved pairings, most recently changed first. */
  pairings = $state.raw<Pairing[]>([]);
  /** The saved pairing these settings came from, if any. */
  pairingId = $state<number | null>(loadPairingId());

  get pairing(): Pairing | undefined {
    return this.pairings.find((p) => p.id === this.pairingId);
  }

  /** True when the settings differ from the saved pairing they came from. */
  get edited(): boolean {
    const p = this.pairing;
    return !!p && !samePairing(this.s, parsePairing(p.settings));
  }

  private setPairingId(id: number | null) {
    this.pairingId = id;
    try {
      if (id === null) localStorage.removeItem(PAIRING_KEY);
      else localStorage.setItem(PAIRING_KEY, String(id));
    } catch {
      /* not persisted */
    }
  }

  async loadPairings() {
    this.pairings = await api.listPairings();
    if (this.pairingId !== null && !this.pairing) this.setPairingId(null);
  }

  /**
   * Switch to a saved pairing, keeping the sample text. Returns the fonts it
   * names that aren't in the library any more.
   */
  openPairing(p: Pairing, families: FamilySummary[]): string[] {
    const saved = parsePairing(p.settings);
    const located = locatePairing(saved, families);
    const missing = (['heading', 'body'] as Role[])
      .filter((r) => saved[r] && !located[r])
      .map((r) => saved[r]!.family);
    this.s = { ...located, texts: this.s.texts };
    this.setPairingId(p.id);
    this.save();
    return missing;
  }

  /** Save the current settings: over the open pairing, or as a new one. */
  async savePairing(name: string, asNew: boolean): Promise<Pairing> {
    const id = asNew ? null : this.pairingId;
    const saved = await api.savePairing(id, name, pairingOf(this.s));
    this.pairings = [saved, ...this.pairings.filter((p) => p.id !== saved.id)];
    this.setPairingId(saved.id);
    return saved;
  }

  async renamePairing(p: Pairing, name: string) {
    const saved = await api.savePairing(p.id, name, parsePairing(p.settings));
    this.pairings = this.pairings.map((x) => (x.id === p.id ? saved : x));
  }

  async deletePairing(p: Pairing) {
    await api.deletePairing(p.id);
    this.pairings = this.pairings.filter((x) => x.id !== p.id);
    if (this.pairingId === p.id) this.setPairingId(null);
  }

  /** Stop tracking a saved pairing; the settings stay as they are. */
  detach() {
    this.setPairingId(null);
  }

  save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.s));
    } catch {
      /* not persisted; still applies this session */
    }
  }

  /**
   * Persist text being typed without touching reactive state: rewriting a
   * block the browser is editing can garble it. Committed to state on blur.
   */
  saveDraft(id: string, text: string) {
    try {
      localStorage.setItem(KEY, JSON.stringify({ ...this.s, texts: { ...this.s.texts, [id]: text } }));
    } catch {
      /* not persisted */
    }
  }

  setFont(role: Role, choice: FontChoice) {
    this.s[role] = choice;
    this.save();
  }

  /** Pick a family for a role, choosing a sensible face within it. */
  setFamily(role: Role, family: FamilySummary) {
    const face = faceNear(family.faces, ROLE_WEIGHT[role]) ?? family.faces[0];
    if (face) this.setFont(role, { family: family.name, faceId: face.id });
  }

  /**
   * Make sure both roles point at faces that still exist, falling back to
   * `fallback` (the first family, or whatever the inspector has open).
   */
  resolve(families: FamilySummary[], fallback?: FamilySummary) {
    for (const role of ['heading', 'body'] as Role[]) {
      const c = this.s[role];
      const found = locate(c, role, families);
      if (found && found.faceId !== c?.faceId) this.setFont(role, found);
      else if (!found) {
        const fam = fallback ?? families[0];
        if (fam) this.setFamily(role, fam);
      }
    }
  }

  reset(keepFonts = true) {
    const d = defaultSettings();
    if (keepFonts) {
      d.heading = this.s.heading;
      d.body = this.s.body;
    }
    this.s = d;
    this.save();
  }

  resetText() {
    this.s.texts = defaultSettings().texts;
    this.textVersion++;
    this.save();
  }

  allStyles(on: boolean) {
    for (const st of STYLES) this.s.styles[st.key].on = on;
    this.save();
  }
}

export const viewer = new ViewerState();
