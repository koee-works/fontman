<script lang="ts" generics="T">
  import Icon from '../ui/Icon.svelte';
  import Segmented from '../ui/Segmented.svelte';
  import type { FacetOption, FacetValue, SetFacet } from '../query';

  // A facet's options as a checklist, each with how many fonts it would show
  // alongside the other filters. Long lists get a search field, and lists
  // where a font can have several values (tags, scripts) can require all of
  // the picked ones instead of any.
  interface Props {
    facet: SetFacet<T>;
    options: (FacetOption & { count: number })[];
    value: FacetValue | undefined;
    onChange: (value: FacetValue | undefined) => void;
  }
  let { facet, options, value, onChange }: Props = $props();

  let search = $state('');
  let list = $state<HTMLElement>();

  const picked = $derived(value?.kind === 'set' ? value.values : []);
  const mode = $derived(value?.kind === 'set' ? value.mode : 'any');
  const shown = $derived.by(() => {
    const s = search.trim().toLowerCase();
    if (!s) return options;
    return options.filter(
      (o) => o.label.toLowerCase().includes(s) || o.value.toLowerCase().includes(s) || o.group?.toLowerCase().includes(s),
    );
  });

  function toggle(v: string) {
    const next = picked.includes(v) ? picked.filter((x) => x !== v) : [...picked, v];
    onChange(next.length ? { kind: 'set', values: next, mode } : undefined);
  }

  function setMode(m: 'any' | 'all') {
    if (picked.length) onChange({ kind: 'set', values: picked, mode: m });
  }

  /** Up and Down move between options; the search field hands off to the list. */
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    const items = [...(list?.querySelectorAll<HTMLButtonElement>('button[role="checkbox"]') ?? [])];
    if (!items.length) return;
    e.preventDefault();
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = at === -1 ? (e.key === 'ArrowDown' ? 0 : items.length - 1) : at + (e.key === 'ArrowDown' ? 1 : -1);
    items[Math.max(0, Math.min(items.length - 1, next))]?.focus();
  }

  const fmt = (n: number) => n.toLocaleString();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel" onkeydown={onKeydown}>
  {#if facet.searchable && options.length > 8}
    <label class="find">
      <Icon name="search" size={14} />
      <span class="visually-hidden">Find {facet.label.toLowerCase()}</span>
      <input bind:value={search} placeholder="Find {facet.label.toLowerCase()}" spellcheck="false" autocomplete="off" />
    </label>
  {/if}

  {#if facet.modes}
    <div class="mode">
      <span>Show fonts with</span>
      <Segmented
        label="Match"
        items={[
          { value: 'any', label: 'Any' },
          { value: 'all', label: 'All' },
        ]}
        value={mode}
        onValueChange={setMode}
      />
    </div>
  {/if}

  <div class="options" bind:this={list} role="group" aria-label={facet.label}>
    {#each shown as o, i (o.value)}
      {#if o.group && o.group !== shown[i - 1]?.group}
        <p class="group">{o.group}</p>
      {/if}
      {@const on = picked.includes(o.value)}
      <button
        type="button"
        role="checkbox"
        aria-checked={on}
        class="option"
        class:empty={o.count === 0 && !on}
        onclick={() => toggle(o.value)}
      >
        <span class="box" class:on>{#if on}<Icon name="check" size={12} />{/if}</span>
        <span class="label">{o.label}</span>
        <span class="count">{fmt(o.count)}</span>
      </button>
    {:else}
      <p class="none">
        {search ? `No ${facet.label.toLowerCase()} match “${search}”.` : `No fonts have ${facet.label.toLowerCase()} yet.`}
      </p>
    {/each}
  </div>

  {#if picked.length}
    <div class="foot">
      <span>{picked.length} picked</span>
      <button type="button" class="link" onclick={() => onChange(undefined)}>Clear</button>
    </div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    min-width: 240px;
    max-width: 320px;
  }
  .find {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: var(--control);
    margin-bottom: var(--s-1);
    padding: 0 var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    color: var(--muted);
  }
  .find:focus-within {
    border-color: var(--focus);
  }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--ink);
    outline: none;
  }
  .mode {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2);
    padding: var(--s-1) var(--s-2) var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .options {
    max-height: min(340px, 50vh);
    overflow-y: auto;
  }
  .group {
    margin: var(--s-2) var(--s-2) var(--s-1);
    color: var(--muted);
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
  }
  .group:first-child {
    margin-top: var(--s-1);
  }
  .option {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    width: 100%;
    height: 28px;
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    text-align: left;
  }
  .option:hover,
  .option:focus-visible {
    background: var(--hover);
    outline: none;
  }
  .option.empty {
    opacity: 0.5;
  }
  .box {
    display: grid;
    place-items: center;
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: var(--r-1);
    border: 1.5px solid var(--line);
    color: var(--cyan-on);
  }
  .box.on {
    background: var(--cyan-solid);
    border-color: var(--cyan-solid);
  }
  .box :global(svg) {
    stroke-width: 2.25;
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    color: var(--muted);
    font-size: var(--text-1);
    font-variant-numeric: tabular-nums;
  }
  .none {
    margin: 0;
    padding: var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: var(--s-1);
    padding: var(--s-2) var(--s-2) var(--s-1);
    border-top: 1px solid var(--faint);
    color: var(--muted);
    font-size: var(--text-1);
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
