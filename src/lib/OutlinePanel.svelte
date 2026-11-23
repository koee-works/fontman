<script lang="ts">
  import { api, type GlyphOutline, type Outlines } from './api';
  import { specimen } from './fonts';
  import IconButton from './ui/IconButton.svelte';
  import NumberField from './ui/NumberField.svelte';
  import Slider from './ui/Slider.svelte';

  // Outlines & metrics: a panel docked under the font list, as Zed docks its
  // terminal. It draws the open font's glyphs from their outlines, at any
  // size, on the font's own vertical metrics (ascender, cap height, x-height,
  // baseline, descender), with the points and control handles that shape
  // each curve. Click a glyph for its spacing. The rendered column shows
  // the same text as the system draws it at reading sizes.
  interface Props {
    faceId: string | null;
    /** "Inter Regular", for the header. */
    title: string;
    variable: boolean;
    /** Height in px, set by dragging the top edge. */
    height: number;
    onResize: (px: number) => void;
    onClose: () => void;
  }
  let { faceId, title, variable, height, onResize, onClose }: Props = $props();

  const load = <T,>(k: string, d: T): T => {
    try {
      const v = localStorage.getItem(k);
      return v === null ? d : (JSON.parse(v) as T);
    } catch {
      return d;
    }
  };
  const save = (k: string, v: unknown) => {
    try {
      localStorage.setItem(k, JSON.stringify(v));
    } catch {
      /* not persisted */
    }
  };

  let text = $state(load('outline-text', 'Hamburg'));
  let filled = $state(load('outline-filled', false));
  let points = $state(load('outline-points', true));
  let lines = $state(load('outline-metrics', true));
  let smooth = $state(load('outline-smooth', true));
  /** Pixels per em, or null to fit the panel's height. */
  let size = $state<number | null>(load('outline-size', null));
  $effect(() => save('outline-text', text));
  $effect(() => save('outline-filled', filled));
  $effect(() => save('outline-points', points));
  $effect(() => save('outline-metrics', lines));
  $effect(() => save('outline-smooth', smooth));
  $effect(() => save('outline-size', size));

  let data = $state<Outlines | null>(null);
  let error = $state<string | null>(null);
  let selected = $state(0);
  let hovered = $state<number | null>(null);

  // Fetch after typing settles; drop answers for text or fonts since changed.
  let seq = 0;
  $effect(() => {
    const id = faceId;
    const t = text || ' ';
    const n = ++seq;
    if (!id) {
      data = null;
      return;
    }
    const timer = setTimeout(() => {
      api
        .glyphOutlines(id, t)
        .then((o) => {
          if (n !== seq) return;
          data = o;
          error = null;
          if (selected >= o.glyphs.length) selected = 0;
        })
        .catch((e) => n === seq && (error = String(e)));
    }, 120);
    return () => clearTimeout(timer);
  });

  // Geometry. Everything below is in screen pixels; font units are scaled by
  // `k` and flipped, since fonts measure y upward.
  let canvasHeight = $state(200);
  const PAD = 24;
  const GUTTER = 96;
  const m = $derived(data?.metrics);
  const top = $derived(
    m ? Math.max(m.ascender, m.capHeight ?? 0, ...(data!.glyphs.map((g) => g.bbox?.[3] ?? 0))) : 0,
  );
  const bottom = $derived(
    m ? Math.min(m.descender, ...(data!.glyphs.map((g) => g.bbox?.[1] ?? 0))) : 0,
  );
  const fit = $derived(m ? Math.max(40, ((canvasHeight - PAD * 2) / (top - bottom)) * m.unitsPerEm) : 200);
  const em = $derived(Math.round(size ?? fit));
  const k = $derived(m ? em / m.unitsPerEm : 1);
  const X = (u: number, at: number) => GUTTER + (at + u) * k;
  const Y = (u: number) => PAD + (top - u) * k;
  const starts = $derived.by(() => {
    const out: number[] = [];
    let x = 0;
    for (const g of data?.glyphs ?? []) {
      out.push(x);
      x += g.advance || (m ? m.unitsPerEm * 0.5 : 500);
    }
    return { at: out, total: x };
  });
  const width = $derived(GUTTER + starts.total * k + PAD * 2);
  const svgHeight = $derived(PAD * 2 + (top - bottom) * k);

  interface Line { label: string; value: number; strong?: boolean }
  const metricLines = $derived.by((): Line[] => {
    if (!m) return [];
    const l: Line[] = [{ label: 'Ascender', value: m.ascender }];
    if (m.capHeight != null) l.push({ label: 'Cap height', value: m.capHeight });
    if (m.xHeight != null) l.push({ label: 'x-height', value: m.xHeight });
    l.push({ label: 'Baseline', value: 0, strong: true }, { label: 'Descender', value: m.descender });
    return l;
  });

  const current = $derived<GlyphOutline | undefined>(data?.glyphs[hovered ?? selected]);
  const hex = (c: string) => 'U+' + c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0');

  // Dragging the top edge sets the panel's height.
  function resize(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const start = e.clientY;
    const from = height;
    el.setPointerCapture(e.pointerId);
    document.body.classList.add('fm-resizing-y');
    const move = (ev: PointerEvent) => onResize(from + (start - ev.clientY));
    const up = () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      el.removeEventListener('pointercancel', up);
      document.body.classList.remove('fm-resizing-y');
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
    el.addEventListener('pointercancel', up);
  }
  function resizeKey(e: KeyboardEvent) {
    const by = e.shiftKey ? 64 : 16;
    if (e.key === 'ArrowUp') onResize(height + by);
    else if (e.key === 'ArrowDown') onResize(height - by);
    else return;
    e.preventDefault();
  }
</script>

<section class="panel" style:height="{height}px" aria-label="Outlines and metrics">
  <!-- The ARIA window-splitter pattern: a focusable separator. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="edge"
    role="separator"
    aria-orientation="horizontal"
    aria-label="Height of the outlines panel"
    aria-valuenow={height}
    tabindex="0"
    onpointerdown={resize}
    onkeydown={resizeKey}
  ></div>

  <header>
    <h2>Outlines & metrics</h2>
    {#if faceId}<span class="font">{title}</span>{/if}
    <input
      class="text"
      bind:value={text}
      maxlength="32"
      spellcheck="false"
      placeholder="Characters to draw"
      aria-label="Characters to draw"
    />
    <span class="toggles" role="group" aria-label="Show">
      <button class="toggle" aria-pressed={filled} onclick={() => (filled = !filled)}>Fill</button>
      <button class="toggle" aria-pressed={points} onclick={() => (points = !points)}>Points</button>
      <button class="toggle" aria-pressed={lines} onclick={() => (lines = !lines)}>Metrics</button>
      <button class="toggle" aria-pressed={smooth} onclick={() => (smooth = !smooth)}>Smoothing</button>
    </span>
    <span class="zoom">
      <!-- Follows the fitted size until set by hand. -->
      <Slider bind:value={() => em, (v) => (size = v)} min={40} max={2000} step={1} label="Size in pixels per em" />
      <NumberField bind:value={() => em, (v) => (size = v)} min={40} max={4000} unit="px" label="Size in pixels per em" />
      <button class="toggle" aria-pressed={size === null} onclick={() => (size = null)}>Fit</button>
    </span>
    <IconButton icon="x" size="sm" label="Close outlines panel" onclick={onClose} />
  </header>

  {#if !faceId}
    <p class="empty">Open a font from the list to see its outlines.</p>
  {:else if error}
    <p class="empty">Couldn't read this font's outlines: {error}</p>
  {:else}
    <div class="body">
      <div class="canvas" class:crisp={!smooth} bind:clientHeight={canvasHeight}>
        {#if data && m}
          <svg width={width} height={svgHeight} role="img" aria-label="Outlines of {text}">
            {#if lines}
              {#each metricLines as l (l.label)}
                <line
                  class="metric"
                  class:strong={l.strong}
                  x1="0"
                  x2={width}
                  y1={Y(l.value)}
                  y2={Y(l.value)}
                />
                <text class="metric-label" x="8" y={Y(l.value) - 4}>{l.label} <tspan class="num">{l.value}</tspan></text>
              {/each}
            {/if}

            {#each data.glyphs as g, i (i)}
              {@const x0 = X(0, starts.at[i]!)}
              {@const adv = (g.advance || m.unitsPerEm * 0.5) * k}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <g
                class="glyph"
                class:selected={i === selected}
                onclick={() => (selected = i)}
                onpointerenter={() => (hovered = i)}
                onpointerleave={() => (hovered = null)}
              >
                <rect class="advance" x={x0} y={Y(top)} width={adv} height={(top - bottom) * k} />
                {#if g.glyphId === null}
                  <text class="missing" x={x0 + adv / 2} y={Y(0) - 8}>no glyph</text>
                {:else}
                  <path
                    class="outline"
                    class:filled
                    d={g.d}
                    transform="translate({x0} {Y(0)}) scale({k} {-k})"
                  />
                  {#if points}
                    {#each g.handles as h, j (j)}
                      <line class="handle" x1={X(h[0], starts.at[i]!)} y1={Y(h[1])} x2={X(h[2], starts.at[i]!)} y2={Y(h[3])} />
                    {/each}
                    {#each g.offCurve as p, j (j)}
                      <circle class="off" cx={X(p[0], starts.at[i]!)} cy={Y(p[1])} r="2.5" />
                    {/each}
                    {#each g.onCurve as p, j (j)}
                      <rect class="on" x={X(p[0], starts.at[i]!) - 2.5} y={Y(p[1]) - 2.5} width="5" height="5" />
                    {/each}
                  {/if}
                {/if}
              </g>
            {/each}
          </svg>
        {:else}
          <p class="empty">Reading outlines…</p>
        {/if}
      </div>

      <aside>
        {#if m}
          <dl class="facts">
            <dt>Units per em</dt><dd>{m.unitsPerEm}</dd>
            <dt>Ascender</dt><dd>{m.ascender}</dd>
            <dt>Cap height</dt><dd>{m.capHeight ?? '—'}</dd>
            <dt>x-height</dt><dd>{m.xHeight ?? '—'}</dd>
            <dt>Descender</dt><dd>{m.descender}</dd>
            <dt>Line gap</dt><dd>{m.lineGap}</dd>
          </dl>
          {#if m.measured}<p class="note">Cap and x-height measured from H and x; the font doesn't state them.</p>{/if}
          {#if variable}<p class="note">Drawn at the font's default instance.</p>{/if}
        {/if}

        {#if current}
          <h3>
            <span class="glyph-char" use:specimen={faceId}>{current.ch}</span>
            {hex(current.ch)}{#if current.name}<span class="muted"> · {current.name}</span>{/if}
          </h3>
          {#if current.glyphId === null}
            <p class="note">This font has no glyph for it.</p>
          {:else}
            <dl class="facts">
              <dt>Advance</dt><dd>{current.advance}</dd>
              <dt>Left side bearing</dt><dd>{current.lsb}</dd>
              <dt>Right side bearing</dt><dd>{current.bbox ? current.advance - current.bbox[2] : '—'}</dd>
              <dt>Bounds</dt><dd>{current.bbox ? `${current.bbox[2] - current.bbox[0]} × ${current.bbox[3] - current.bbox[1]}` : 'Empty'}</dd>
              <dt>Contours</dt><dd>{current.contours}</dd>
              <dt>Points</dt><dd>{current.onCurve.length} on, {current.offCurve.length} off</dd>
            </dl>
          {/if}
        {/if}

        <h3>As rendered</h3>
        <div class="rendered" class:crisp={!smooth}>
          {#each [11, 14, 18, 28] as px (px)}
            <div class="size-row">
              <span class="px">{px}</span>
              <span class="sample" style:font-size="{px}px" use:specimen={faceId}>{text || ' '}</span>
            </div>
          {/each}
        </div>
      </aside>
    </div>
  {/if}
</section>

<style>
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    min-height: 0;
    /* A grid item sizes to its content by default; a wide drawing scrolls
       inside the panel instead of widening the list's column. */
    min-width: 0;
    container-type: inline-size;
    background: var(--sheet);
    border-top: var(--pane-rule-width) solid var(--faint);
    border-left: var(--pane-rule-width) solid var(--faint);
  }
  .edge {
    position: absolute;
    left: 0;
    right: 0;
    top: -4px;
    height: 7px;
    z-index: var(--z-bar);
    cursor: row-resize;
    touch-action: none;
  }
  .edge::after {
    content: '';
    position: absolute;
    inset: 3px 0 2px;
    transition: background-color var(--dur-fast) var(--ease);
  }
  .edge:hover::after,
  .edge:focus-visible::after,
  :global(body.fm-resizing-y) .edge::after {
    background: var(--focus);
  }
  .edge:focus-visible {
    outline: none;
  }
  :global(body.fm-resizing-y) {
    cursor: row-resize;
    user-select: none;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 36px;
    flex: none;
    padding: 0 var(--s-2) 0 var(--s-4);
    border-bottom: 1px solid var(--faint);
    color: var(--muted);
    font-size: var(--text-1);
    white-space: nowrap;
    overflow: hidden;
  }
  h2 {
    margin: 0;
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--ink);
  }
  .font {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .text {
    width: 140px;
    flex: none;
    height: 24px;
    margin-left: auto;
    padding: 0 var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    color: var(--ink);
    font: inherit;
  }
  .toggles,
  .zoom {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .zoom {
    gap: var(--s-2);
    width: 280px;
  }
  .zoom :global(.fm-number) {
    flex: none;
  }
  .toggle {
    height: 24px;
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .toggle:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .toggle[aria-pressed='true'] {
    background: var(--selected);
    color: var(--cyan);
  }

  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px;
  }
  .canvas {
    min-width: 0;
    min-height: 0;
    overflow: auto;
  }
  svg {
    display: block;
    /* Smoothing on: curves anti-aliased at sub-pixel precision. */
    shape-rendering: geometricPrecision;
  }
  /* Smoothing off: edges snap to whole pixels, as unsmoothed text would. */
  .crisp svg {
    shape-rendering: crispEdges;
  }
  .metric {
    stroke: var(--line);
    stroke-width: 1;
  }
  .metric.strong {
    stroke: var(--muted);
  }
  .metric-label {
    fill: var(--muted);
    font: 11px var(--ui);
  }
  .metric-label .num {
    font-variant-numeric: tabular-nums;
  }
  .advance {
    fill: transparent;
    stroke: var(--faint);
    stroke-dasharray: 2 3;
  }
  .glyph:hover .advance {
    fill: var(--hover);
  }
  .glyph.selected .advance {
    fill: var(--selected);
    stroke: color-mix(in srgb, var(--cyan-solid) 50%, transparent);
    stroke-dasharray: none;
  }
  .outline {
    fill: none;
    stroke: var(--ink);
    stroke-width: 1.25;
    vector-effect: non-scaling-stroke;
  }
  .outline.filled {
    fill: var(--ink);
    stroke: none;
  }
  .handle {
    stroke: var(--muted);
    stroke-width: 1;
    opacity: 0.7;
  }
  .on {
    fill: var(--cyan-solid);
  }
  .off {
    fill: var(--sheet);
    stroke: var(--cyan);
    stroke-width: 1.25;
  }
  .missing {
    fill: var(--muted);
    font: 11px var(--ui);
    text-anchor: middle;
  }

  aside {
    min-height: 0;
    overflow-y: auto;
    padding: var(--s-3) var(--s-4);
    border-left: 1px solid var(--faint);
    font-size: var(--text-1);
  }
  h3 {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
    margin: var(--s-4) 0 var(--s-2);
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--ink);
  }
  .glyph-char {
    font-size: 20px;
    font-weight: 400;
    font-synthesis: none;
  }
  .muted {
    color: var(--muted);
    font-weight: 400;
  }
  .facts {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px var(--s-3);
    margin: 0;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .note {
    margin: var(--s-2) 0 0;
    color: var(--muted);
  }
  .rendered .size-row {
    display: grid;
    grid-template-columns: 20px 1fr;
    align-items: baseline;
    gap: var(--s-2);
    white-space: nowrap;
    overflow: hidden;
  }
  .px {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .sample {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink);
    font-synthesis: none;
    /* Smoothing on: grayscale anti-aliasing and kerning-aware layout.
       Browsers honour font-smoothing on macOS; elsewhere the system decides,
       so the outline view is the reliable comparison. */
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
    text-rendering: optimizeLegibility;
  }
  .rendered.crisp .sample {
    -webkit-font-smoothing: none;
    -moz-osx-font-smoothing: auto;
    text-rendering: optimizeSpeed;
  }
  .empty {
    margin: var(--s-4);
    color: var(--muted);
  }

  /* Narrower panels drop, in order: the font's name (the inspector shows
     it), the size slider (the typed size and Fit stay), then sidebar width.
     The close button is never pushed out. */
  header > :global(.fm-icon-btn) {
    flex: none;
  }
  @container (max-width: 860px) {
    .font {
      display: none;
    }
  }
  @container (max-width: 760px) {
    .zoom {
      width: auto;
    }
    .zoom :global(.fm-slider) {
      display: none;
    }
  }
  @container (max-width: 620px) {
    .body {
      grid-template-columns: minmax(0, 1fr) 200px;
    }
    .text {
      width: 96px;
    }
  }
</style>
