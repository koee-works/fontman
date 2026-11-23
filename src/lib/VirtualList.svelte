<script lang="ts" generics="T">
  // Renders only the rows near the viewport.
  //
  // Mounting every row costs ~0.7 ms each in WebKit; at 4,000 families that
  // was a 2.8 s first paint and a ~450 ms re-render on every change. Rows here
  // have variable heights (families expand, the size slider changes all of
  // them), so each row is measured as it appears and unseen rows use an
  // estimate. When a row above the viewport measures differently from its
  // estimate, the scroll position is corrected so content doesn't jump.

  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';

  interface Props {
    items: T[];
    key: (item: T) => string;
    estimate: (item: T) => number;
    row: Snippet<[T]>;
    /** Changing this scrolls back to the top (new search or filter). */
    resetScroll?: unknown;
    /** Changing this discards measured heights (e.g. preview size changed). */
    resetMeasure?: unknown;
    /** Pixels rendered beyond each edge, so fast scrolling doesn't show gaps. */
    overscan?: number;
    label?: string;
  }

  let {
    items,
    key,
    estimate,
    row,
    resetScroll,
    resetMeasure,
    overscan = 800,
    label,
  }: Props = $props();

  let viewport = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let height = $state(0);
  let measured = $state.raw(new Map<string, number>());

  const offsets = $derived.by(() => {
    const out = new Float64Array(items.length + 1);
    for (let i = 0; i < items.length; i++) {
      const it = items[i]!;
      out[i + 1] = out[i]! + (measured.get(key(it)) ?? estimate(it));
    }
    return out;
  });
  const total = $derived(offsets[items.length] ?? 0);

  /** Index of the row containing y. */
  function indexAt(y: number): number {
    let lo = 0;
    let hi = items.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (offsets[mid + 1]! <= y) lo = mid + 1;
      else hi = mid;
    }
    return lo;
  }

  const start = $derived(Math.max(0, indexAt(scrollTop - overscan)));
  const end = $derived(Math.min(items.length, indexAt(scrollTop + height + overscan) + 1));
  const visible = $derived(items.slice(start, end));

  $effect(() => {
    resetScroll;
    if (viewport) viewport.scrollTop = 0;
    scrollTop = 0;
  });

  $effect(() => {
    resetMeasure;
    untrack(() => (measured = new Map()));
  });

  // Measurements arrive from a ResizeObserver; apply them in one batch per
  // frame so a screenful of rows causes one offset recompute, not twenty.
  let pending = new Map<string, number>();
  let scheduled = false;

  function flush() {
    scheduled = false;
    if (!pending.size || !viewport) return;
    const next = new Map(measured);
    let shiftAbove = 0;
    const top = viewport.scrollTop;
    for (const [k, h] of pending) {
      const prev = next.get(k);
      const i = items.findIndex((it) => key(it) === k);
      if (i < 0) continue;
      const before = prev ?? estimate(items[i]!);
      if (Math.abs(before - h) < 0.5) continue;
      // A row entirely above the visible top moves everything we're looking at.
      if (offsets[i + 1]! <= top) shiftAbove += h - before;
      next.set(k, h);
    }
    pending = new Map();
    measured = next;
    if (shiftAbove !== 0) {
      viewport.scrollTop = top + shiftAbove;
      scrollTop = viewport.scrollTop;
    }
  }

  const observer = new ResizeObserver((entries) => {
    for (const e of entries) {
      const k = (e.target as HTMLElement).dataset.key;
      if (k) pending.set(k, e.borderBoxSize[0]?.blockSize ?? e.contentRect.height);
    }
    if (!scheduled) {
      scheduled = true;
      requestAnimationFrame(flush);
    }
  });

  function measure(node: HTMLElement) {
    observer.observe(node);
    return { destroy: () => observer.unobserve(node) };
  }
</script>

<div
  class="viewport"
  role="list"
  aria-label={label}
  bind:this={viewport}
  bind:clientHeight={height}
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  <div class="canvas" style:height="{total}px">
    <div class="window" style:transform="translateY({offsets[start] ?? 0}px)">
      {#each visible as item (key(item))}
        <div role="listitem" data-key={key(item)} use:measure>
          {@render row(item)}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .viewport {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    /* We correct scroll position ourselves when rows above are measured. */
    overflow-anchor: none;
  }
  .canvas {
    position: relative;
  }
  .window {
    position: absolute;
    inset: 0 0 auto 0;
  }
</style>
