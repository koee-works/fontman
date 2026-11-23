<script lang="ts">
  import { api } from './api';
  import { fontStack } from './fonts';

  interface Props {
    faceId: string;
    variation: string;
    /** CSS font-feature-settings from the Features tab. */
    features?: string;
  }
  let { faceId, variation, features = 'normal' }: Props = $props();

  const CELL = 52;
  const OVERSCAN = 3;

  let ranges = $state<[number, number][]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  let viewport = $state<HTMLDivElement>();
  let width = $state(0);
  let height = $state(0);
  let scrollTop = $state(0);
  let picked = $state<number | null>(null);

  $effect(() => {
    const id = faceId;
    loading = true;
    error = null;
    picked = null;
    ranges = [];
    let stale = false;
    api
      .faceCoverage(id)
      .then((r) => {
        if (!stale) ranges = r;
      })
      .catch((e) => {
        if (!stale) error = String(e);
      })
      .finally(() => {
        if (!stale) loading = false;
      });
    if (viewport) viewport.scrollTop = 0;
    return () => (stale = true);
  });

  // Prefix sums over range lengths turn "cell N" into a codepoint in log time,
  // without ever materialising the flat list of tens of thousands of codepoints.
  const offsets = $derived.by(() => {
    const out: number[] = [];
    let total = 0;
    for (const [a, b] of ranges) {
      out.push(total);
      total += b - a + 1;
    }
    out.push(total);
    return out;
  });
  const total = $derived(offsets[offsets.length - 1] ?? 0);

  function codepointAt(i: number): number {
    let lo = 0;
    let hi = ranges.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if ((offsets[mid] ?? 0) <= i) lo = mid;
      else hi = mid - 1;
    }
    const range = ranges[lo]!;
    return range[0] + (i - (offsets[lo] ?? 0));
  }

  // Keep a right gutter matching the panel padding; the scrollbar sits past it.
  const GUTTER = 20;
  const cols = $derived(Math.max(1, Math.floor((width - GUTTER) / CELL)));
  // Cells stretch so the grid spans the content width exactly: flush with the
  // panel's left padding and its right gutter, never a ragged edge.
  const cellW = $derived(Math.max(CELL, (width - GUTTER) / cols));
  const rows = $derived(Math.ceil(total / cols));
  const first = $derived(Math.max(0, Math.floor(scrollTop / CELL) - OVERSCAN));
  const last = $derived(Math.min(rows, Math.ceil((scrollTop + height) / CELL) + OVERSCAN));

  const visible = $derived.by(() => {
    const cells: { i: number; cp: number }[] = [];
    for (let r = first; r < last; r++) {
      for (let c = 0; c < cols; c++) {
        const i = r * cols + c;
        if (i >= total) break;
        cells.push({ i, cp: codepointAt(i) });
      }
    }
    return cells;
  });

  const hex = (cp: number) => 'U+' + cp.toString(16).toUpperCase().padStart(4, '0');
</script>

<section class="glyphs" aria-label="Character set">
  <div class="summary">
    {#if loading}
      Reading character set…
    {:else if error}
      <span class="error">Could not read the character set: {error}</span>
    {:else}
      {total.toLocaleString()} characters
      {#if picked !== null}
        <span class="picked">
          <span class="big" style:font-family={fontStack(faceId)} style:font-variation-settings={variation}
          style:font-feature-settings={features}>
            {String.fromCodePoint(picked)}
          </span>
          <code>{hex(picked)}</code>
        </span>
      {/if}
    {/if}
  </div>

  <div
    class="viewport"
    bind:this={viewport}
    bind:clientWidth={width}
    bind:clientHeight={height}
    onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
  >
    <div class="canvas" style:height="{rows * CELL}px">
      {#each visible as { i, cp } (i)}
        <button
          class="cell"
          class:active={picked === cp}
          style:top="{Math.floor(i / cols) * CELL}px"
          style:left="{(i % cols) * cellW}px"
          style:width="{cellW}px"
          style:font-family={fontStack(faceId)}
          style:font-variation-settings={variation}
          style:font-feature-settings={features}
          title={hex(cp)}
          onclick={() => (picked = cp)}
        >
          {String.fromCodePoint(cp)}
        </button>
      {/each}
    </div>
  </div>
</section>

<style>
  .glyphs {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .summary {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    min-height: 48px;
    color: var(--muted);
    border-bottom: 1px solid var(--faint);
  }
  .error {
    color: var(--red);
  }
  .picked {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--ink);
  }
  code {
    font-family: var(--mono);
    font-size: var(--text-1);
    padding: 2px var(--s-2);
    border-radius: var(--r-1);
    background: var(--hover);
  }
  .big {
    font-size: 32px;
    line-height: 1;
    font-synthesis: none;
  }
  .viewport {
    flex: 1;
    min-height: 240px;
    overflow-y: auto;
    position: relative;
    margin-right: calc(var(--s-5) * -1);
  }
  .canvas {
    position: relative;
  }
  .cell {
    position: absolute;
    height: 52px;
    display: grid;
    place-items: center;
    font-size: 24px;
    line-height: 1;
    color: var(--ink);
    font-synthesis: none;
  }
  .cell:hover {
    background: var(--hover);
  }
  .cell.active {
    background: var(--cyan);
    color: var(--cyan-ink);
  }
</style>
