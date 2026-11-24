// Compare: two styles side by side, or one over the other. Remembered
// between launches (guarded: storage can be unavailable).

import type { FamilySummary } from './api';
import { faceNear } from './viewer/state.svelte';

export interface Pick {
  family: string;
  faceId: string;
}
export type Side = 'left' | 'right';
export type CompareMode = 'side' | 'overlay';

interface Saved {
  left: Pick | null;
  right: Pick | null;
  sample: string;
  size: number;
  mode: CompareMode;
}

const KEY = 'compare';
const DEFAULTS: Saved = { left: null, right: null, sample: '', size: 72, mode: 'side' };

function load(): Saved {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') };
  } catch {
    return { ...DEFAULTS };
  }
}

class CompareState {
  s = $state<Saved>(load());

  save() {
    try {
      localStorage.setItem(KEY, JSON.stringify(this.s));
    } catch {
      /* not persisted */
    }
  }

  set(side: Side, pick: Pick) {
    this.s[side] = pick;
    this.save();
  }

  /** Pick a family for a side, at the style nearest the other side's weight. */
  setFamily(side: Side, fam: FamilySummary, nearWeight = 400) {
    const face = faceNear(fam.faces, nearWeight) ?? fam.faces[0];
    if (face) this.set(side, { family: fam.name, faceId: face.id });
  }

  swap() {
    [this.s.left, this.s.right] = [this.s.right, this.s.left];
    this.save();
  }

  /**
   * Make both sides point at styles that exist. A missing left takes
   * `preferred` (the style open in the inspector) or the first family; a
   * missing right takes another style of the left's family, or the next family.
   */
  resolve(families: FamilySummary[], preferred?: Pick) {
    const exists = (p: Pick | null) => !!p && families.some((f) => f.faces.some((x) => x.id === p.faceId));
    if (!exists(this.s.left)) {
      const fam = families[0];
      this.s.left = preferred ?? (fam && fam.faces[0] ? { family: fam.name, faceId: fam.faces[0].id } : null);
    }
    if (!exists(this.s.right) && this.s.left) {
      const leftFam = families.find((f) => f.name === this.s.left!.family);
      const other = leftFam?.faces.find((x) => x.id !== this.s.left!.faceId);
      const next = families.find((f) => f.name !== this.s.left!.family);
      this.s.right = other
        ? { family: leftFam!.name, faceId: other.id }
        : next && next.faces[0]
          ? { family: next.name, faceId: next.faces[0].id }
          : null;
    }
    this.save();
  }
}

export const compare = new CompareState();
