// The command palette's model: commands, and fuzzy matching over them.

export interface Command {
  /** Stable id, for remembering recent commands. */
  id: string;
  /** Shown muted before the title: "Library", "View". */
  group: string;
  title: string;
  /** Display form, e.g. "Ctrl+B" or "⌘B". */
  shortcut?: string;
  /** Extra words that should match, never shown. */
  keywords?: string;
  run: () => void;
}

export interface Match {
  score: number;
  /** Positions in the title that matched, for highlighting. */
  indices: number[];
}

const isBoundary = (text: string, i: number) =>
  i === 0 || /[\s\-_/:.(]/.test(text[i - 1]!) || (/[a-z]/.test(text[i - 1]!) && /[A-Z]/.test(text[i]!));

/**
 * Fuzzy-match `query` against `text`: every query character must appear in
 * order, ignoring case and spaces. Matches at word starts and runs of
 * consecutive characters score higher; gaps and long texts score lower.
 * Returns null when the query doesn't match.
 */
export function fuzzy(query: string, text: string): Match | null {
  const q = query.toLowerCase().replace(/\s+/g, '');
  if (!q) return { score: 0, indices: [] };
  const t = text.toLowerCase();
  // Greedy from each candidate start, keep the best: cheap and good enough
  // for a few hundred short strings.
  let best: Match | null = null;
  for (let start = t.indexOf(q[0]!); start !== -1; start = t.indexOf(q[0]!, start + 1)) {
    const indices: number[] = [];
    let score = 0;
    let ti = start;
    let prev = -2;
    for (const ch of q) {
      while (ti < t.length && t[ti] !== ch) ti++;
      if (ti >= t.length) {
        indices.length = 0;
        break;
      }
      score += 1;
      if (ti === prev + 1) score += 3; // consecutive
      if (isBoundary(text, ti)) score += 4; // word start
      if (prev >= 0) score -= Math.min(ti - prev - 1, 5) * 0.5; // gap
      indices.push(ti);
      prev = ti;
      ti++;
    }
    if (indices.length !== q.length) continue;
    score -= indices[0]! * 0.1 + text.length * 0.02;
    if (!best || score > best.score) best = { score, indices };
  }
  return best;
}

/** Match a command on its title, or failing that its group and keywords. */
export function matchCommand(query: string, c: Command): Match | null {
  const title = fuzzy(query, c.title);
  if (title) return title;
  const other = fuzzy(query, `${c.group} ${c.title} ${c.keywords ?? ''}`);
  // Well below any title match: keywords only decide when the title says nothing.
  return other ? { score: other.score * 0.5 - 5, indices: [] } : null;
}

/** Split text into runs of matched and unmatched characters, for rendering. */
export function highlight(text: string, indices: number[]): { text: string; hit: boolean }[] {
  const hits = new Set(indices);
  const out: { text: string; hit: boolean }[] = [];
  for (let i = 0; i < text.length; i++) {
    const hit = hits.has(i);
    const last = out[out.length - 1];
    if (last && last.hit === hit) last.text += text[i];
    else out.push({ text: text[i]!, hit });
  }
  return out;
}
