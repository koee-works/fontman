<script lang="ts" generics="T">
  import { Slider } from 'bits-ui';
  import NumberField from '../ui/NumberField.svelte';
  import type { FacetValue, RangeFacet } from '../query';

  // A numeric range with two handles, typed values, quick picks, and a bar
  // per step showing how many fonts reach it with the other filters applied.
  interface Props {
    facet: RangeFacet<T>;
    /** Items passing every other filter, for the bars. */
    pool: readonly T[];
    value: FacetValue | undefined;
    onChange: (value: FacetValue | undefined) => void;
  }
  let { facet, pool, value, onChange }: Props = $props();

  const lo = $derived(value?.kind === 'range' ? value.min : facet.min);
  const hi = $derived(value?.kind === 'range' ? value.max : facet.max);

  function set(min: number, max: number) {
    min = Math.max(facet.min, Math.min(min, facet.max));
    max = Math.max(facet.min, Math.min(max, facet.max));
    if (min > max) [min, max] = [max, min];
    onChange(min <= facet.min && max >= facet.max ? undefined : { kind: 'range', min, max });
  }

  const steps = $derived.by(() => {
    const out: number[] = [];
    for (let v = facet.min; v <= facet.max; v += facet.step) out.push(v);
    return out;
  });
  const bars = $derived.by(() => {
    const counts = steps.map(() => 0);
    for (const item of pool) {
      const [a, b] = facet.span(item);
      steps.forEach((s, i) => {
        if (b >= s && a <= s) counts[i]!++;
      });
    }
    return counts;
  });
  const peak = $derived(Math.max(1, ...bars));
  const fmt = (n: number) => (facet.format ?? String)(n);

  // Number fields edit a copy, applied when they change.
  let minDraft = $state(0);
  let maxDraft = $state(0);
  $effect.pre(() => {
    minDraft = lo;
    maxDraft = hi;
  });
  $effect(() => {
    if (minDraft !== lo || maxDraft !== hi) set(minDraft, maxDraft);
  });
</script>

<div class="panel">
  <div class="head">
    <span class="title">{facet.label}</span>
    <span class="readout">
      {lo <= facet.min && hi >= facet.max ? 'Any' : lo === hi ? fmt(lo) : `${fmt(lo)} to ${fmt(hi)}`}
    </span>
  </div>

  <!-- One bar per step, centred under where the handle sits for that value. -->
  <div class="bars" aria-hidden="true">
    {#each bars as n, i (i)}
      <span
        class="bar"
        class:in={steps[i]! >= lo && steps[i]! <= hi}
        style:left="calc(8px + (100% - 16px) * {steps.length > 1 ? i / (steps.length - 1) : 0.5})"
        style:width="min(16px, calc((100% - 16px) / {Math.max(1, steps.length - 1)} - 2px))"
        style:height="{Math.max(n ? 8 : 2, (n / peak) * 100)}%"
      ></span>
    {/each}
  </div>

  <Slider.Root
    type="multiple"
    value={[lo, hi]}
    min={facet.min}
    max={facet.max}
    step={facet.step}
    onValueChange={(v) => set(v[0] ?? facet.min, v[1] ?? facet.max)}
    class="fm-slider"
  >
    {#snippet children({ thumbs })}
      <span class="fm-slider-track">
        <Slider.Range class="fm-slider-range" />
      </span>
      {#each thumbs as index (index)}
        <Slider.Thumb
          {index}
          class="fm-slider-thumb"
          aria-label={index === 0 ? `Lowest ${facet.label.toLowerCase()}` : `Highest ${facet.label.toLowerCase()}`}
        />
      {/each}
    {/snippet}
  </Slider.Root>

  <div class="ends">
    <span>
      <NumberField bind:value={minDraft} min={facet.min} max={facet.max} step={facet.step} unit={facet.unit} label="Lowest {facet.label.toLowerCase()}" />
    </span>
    <span>
      <NumberField bind:value={maxDraft} min={facet.min} max={facet.max} step={facet.step} unit={facet.unit} label="Highest {facet.label.toLowerCase()}" />
    </span>
  </div>

  {#if facet.presets?.length}
    <div class="presets" role="group" aria-label="Quick picks">
      {#each facet.presets as p (p.label)}
        <button
          type="button"
          class="preset"
          aria-pressed={lo === p.min && hi === p.max}
          onclick={() => set(p.min, p.max)}
        >
          {p.label}
        </button>
      {/each}
    </div>
  {/if}

  {#if value}
    <div class="foot">
      <button type="button" class="link" onclick={() => onChange(undefined)}>Clear</button>
    </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    width: 280px;
    padding: var(--s-2);
  }
  .head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-2);
  }
  .title {
    font-weight: var(--weight-strong);
  }
  .bars {
    position: relative;
    height: 40px;
    margin-top: var(--s-1);
  }
  .bar {
    position: absolute;
    bottom: 0;
    transform: translateX(-50%);
    border-radius: 2px 2px 0 0;
    background: var(--faint);
    transition: background-color var(--dur-fast) var(--ease);
  }
  .bar.in {
    background: var(--cyan-soft);
    box-shadow: inset 0 2px 0 var(--cyan);
  }
  .ends {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2);
    font-size: var(--text-1);
  }
  .readout {
    color: var(--cyan);
    font-size: var(--text-1);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-1);
  }
  .preset {
    height: var(--control-sm);
    padding: 0 var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    font-size: var(--text-1);
  }
  .preset:hover {
    background: var(--hover);
  }
  .preset[aria-pressed='true'] {
    border-color: var(--cyan-solid);
    background: var(--cyan-soft);
    color: var(--cyan);
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    padding-top: var(--s-2);
    border-top: 1px solid var(--faint);
  }
  .link {
    color: var(--cyan);
    font-size: var(--text-1);
    font-weight: 500;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
