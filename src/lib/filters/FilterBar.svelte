<script lang="ts" generics="T">
  import { Popover } from 'bits-ui';
  import Icon from '../ui/Icon.svelte';
  import RangePanel from './RangePanel.svelte';
  import SetPanel from './SetPanel.svelte';
  import SortControl from './SortControl.svelte';
  import {
    activeCount,
    applyFacets,
    describeFacet,
    facetOptions,
    isFacetActive,
    type Facet,
    type FacetValue,
    type SortKey,
    type SortState,
  } from '../query';
  import type { QueryState } from '../query-state.svelte';

  // One row under the toolbar: a chip per filter, the result count and the
  // sort order. A chip names its filter and, once set, what it's set to, so
  // the row reads as a sentence of what's shown. Each opens a panel with the
  // choices and how many fonts each would leave.
  interface Props {
    /** Items after search, before these filters. */
    items: readonly T[];
    facets: readonly Facet<T>[];
    query: QueryState;
    sorts: readonly SortKey<T>[];
    /** The order in effect, which may differ from the saved one while searching. */
    sort: SortState;
    onSort: (sort: SortState) => void;
    searching: boolean;
    /** How many are shown, and the word for them. */
    shown: number;
    noun: [one: string, many: string];
  }
  let { items, facets, query, sorts, sort, onSort, searching, shown, noun }: Props = $props();

  const set = (id: string, v: FacetValue | undefined) => query.setFilters({ ...query.filters, [id]: v });
  const active = $derived(activeCount(facets, query.filters));
  const word = (n: number) => (n === 1 ? noun[0] : noun[1]);
</script>

<div class="bar" role="toolbar" aria-label="Filters and sorting">
  <div class="chips">
    <span class="lead" aria-hidden="true"><Icon name="filter" size={14} /></span>
    {#each facets as facet (facet.id)}
      {@const v = query.filters[facet.id]}
      {@const on = isFacetActive(facet, v)}
      <span class="chip-wrap" class:on>
        <Popover.Root>
          <Popover.Trigger
            class="fm-filter-chip"
            data-active={on ? '' : undefined}
            aria-label={on ? `${facet.label}: ${describeFacet(facet, v)}. Change` : `Filter by ${facet.label.toLowerCase()}`}
          >
            <Icon name={facet.icon} size={14} />
            <span>{facet.label}</span>
            {#if on}<span class="value">{describeFacet(facet, v)}</span>{/if}
            <Icon name="chevron-down" size={12} />
          </Popover.Trigger>
          <Popover.Portal>
            <Popover.Content class="fm-menu fm-filter-pop" sideOffset={4} align="start">
              {#if facet.kind === 'set'}
                <SetPanel
                  {facet}
                  options={facetOptions(items, facets, query.filters, facet)}
                  value={v}
                  onChange={(x) => set(facet.id, x)}
                />
              {:else}
                <RangePanel
                  {facet}
                  pool={applyFacets(items, facets, query.filters, facet.id)}
                  value={v}
                  onChange={(x) => set(facet.id, x)}
                />
              {/if}
            </Popover.Content>
          </Popover.Portal>
        </Popover.Root>
        {#if on}
          <button
            type="button"
            class="clear"
            aria-label="Remove the {facet.label.toLowerCase()} filter"
            onclick={() => set(facet.id, undefined)}
          >
            <Icon name="x" size={12} />
          </button>
        {/if}
      </span>
    {/each}
    {#if active > 1}
      <button type="button" class="clear-all" onclick={() => query.clear()}>Clear all</button>
    {/if}
  </div>

  <div class="end">
    <span class="count" aria-live="polite">
      {#if active && shown !== items.length}
        {shown.toLocaleString()} of {items.length.toLocaleString()} {word(items.length)}
      {:else}
        {shown.toLocaleString()} {word(shown)}
      {/if}
    </span>
    <SortControl keys={sorts} value={sort} {searching} onChange={onSort} />
  </div>
</div>

<style>
  .bar {
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
    padding: var(--s-2) var(--s-4) var(--s-2) var(--s-5);
    border-bottom: 1px solid var(--faint);
    background: var(--sheet);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-1);
    flex: 1;
    min-width: 0;
  }
  .lead {
    display: grid;
    place-items: center;
    width: 20px;
    height: var(--control-sm);
    margin-right: 2px;
    color: var(--muted);
  }
  .end {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    flex: none;
  }
  .count {
    color: var(--muted);
    font-size: var(--text-1);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* Shared with the sort control: a quiet pill that fills in when set. */
  :global(.fm-filter-chip) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: var(--control-sm);
    padding: 0 var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    background: var(--sheet);
    color: var(--ink);
    font-size: var(--text-1);
    white-space: nowrap;
    transition: background-color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  :global(.fm-filter-chip:hover),
  :global(.fm-filter-chip[data-state='open']) {
    background: var(--hover);
  }
  :global(.fm-filter-chip svg) {
    color: var(--muted);
    flex: none;
  }
  :global(.fm-filter-chip[data-active]) {
    border-color: var(--cyan-solid);
    background: var(--cyan-soft);
  }
  :global(.fm-filter-pop) {
    min-width: 0;
  }

  .chip-wrap {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
  }
  .chip-wrap.on :global(.fm-filter-chip) {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
    border-right: 0;
    padding-right: var(--s-1);
    max-width: 280px;
  }
  .value {
    min-width: 0;
    color: var(--cyan);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .clear {
    display: grid;
    place-items: center;
    width: 22px;
    height: var(--control-sm);
    border: 1px solid var(--cyan-solid);
    border-left: 0;
    border-radius: 0 var(--r-full) var(--r-full) 0;
    background: var(--cyan-soft);
    color: var(--cyan);
  }
  .clear:hover {
    background: var(--cyan-solid);
    color: var(--cyan-on);
  }
  .clear-all {
    height: var(--control-sm);
    padding: 0 var(--s-2);
    color: var(--cyan);
    font-size: var(--text-1);
    font-weight: 500;
  }
  .clear-all:hover {
    text-decoration: underline;
  }
</style>
