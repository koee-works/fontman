// Motion: how fast the app animates, or whether it does at all. Three
// settings, chosen in Settings → Appearance:
//
//   none      no animation; everything changes at once
//   snappy    quick, the default
//   graceful  slower and softer
//
// Every animation reads its timing from here. CSS uses the tokens written
// onto <html> (--dur-fast for hover feedback, --dur for menus and tabs,
// --dur-pane for panes, --dur-slow for fonts fading in, --ease). Svelte
// transitions use `motion.ms`. With nothing chosen, a system request for
// reduced motion means None.

export type Motion = 'none' | 'snappy' | 'graceful';

export interface Speeds {
  fast: number;
  base: number;
  pane: number;
  slow: number;
  ease: string;
}

export const SPEEDS: Record<Motion, Speeds> = {
  none: { fast: 0, base: 0, pane: 0, slow: 0, ease: 'linear' },
  snappy: { fast: 90, base: 140, pane: 180, slow: 220, ease: 'cubic-bezier(0.2, 0.7, 0.2, 1)' },
  graceful: { fast: 160, base: 280, pane: 380, slow: 520, ease: 'cubic-bezier(0.3, 0.6, 0.2, 1)' },
};

const KEY = 'motion';

function stored(): Motion | null {
  try {
    const v = localStorage.getItem(KEY);
    return v === 'none' || v === 'snappy' || v === 'graceful' ? v : null;
  } catch {
    return null;
  }
}

const reduceQuery =
  typeof matchMedia === 'function' ? matchMedia('(prefers-reduced-motion: reduce)') : null;

class MotionState {
  /** What the person chose, or null to follow the system. */
  choice = $state<Motion | null>(stored());
  /** The system asks for reduced motion. */
  systemReduce = $state(reduceQuery?.matches ?? false);
  /** In effect now. */
  current = $derived<Motion>(this.choice ?? (this.systemReduce ? 'none' : 'snappy'));

  constructor() {
    this.apply();
    reduceQuery?.addEventListener('change', (e) => {
      this.systemReduce = e.matches;
      this.apply();
    });
  }

  set(m: Motion) {
    this.choice = m;
    try {
      localStorage.setItem(KEY, m);
    } catch {
      /* not remembered */
    }
    this.apply();
  }

  apply() {
    if (typeof document === 'undefined') return;
    const m = this.choice ?? (reduceQuery?.matches ? 'none' : 'snappy');
    const s = SPEEDS[m];
    const root = document.documentElement;
    root.dataset.motion = m;
    root.style.setProperty('--dur-fast', `${s.fast}ms`);
    root.style.setProperty('--dur', `${s.base}ms`);
    root.style.setProperty('--dur-pane', `${s.pane}ms`);
    root.style.setProperty('--dur-slow', `${s.slow}ms`);
    root.style.setProperty('--ease', s.ease);
  }
}

export const motion = new MotionState();

/** Milliseconds for a Svelte transition, read when it starts. */
export const ms = (k: keyof Omit<Speeds, 'ease'>) => SPEEDS[motion.current][k];

/**
 * A pane sliding in from, or out to, one edge. The pane keeps its size and
 * its layout slot grows or shrinks through a negative margin on that edge,
 * so the panes beside it move with it and its content never reflows.
 */
export function drawer(node: HTMLElement, { edge }: { edge: 'left' | 'right' | 'bottom' }) {
  const size = edge === 'bottom' ? node.offsetHeight : node.offsetWidth;
  return {
    duration: size ? ms('pane') : 0,
    // Ease out on the way in; the reverse on the way out.
    easing: (t: number) => 1 - (1 - t) ** 3,
    css: (t: number) => `margin-${edge}: ${(-(1 - t) * size).toFixed(1)}px; opacity: ${Math.min(1, 0.4 + t)}`,
  };
}
