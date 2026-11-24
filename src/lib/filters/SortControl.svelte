<script lang="ts" generics="T">
  import { Popover } from 'bits-ui';
  import Icon from '../ui/Icon.svelte';
  import type { SortKey, SortState } from '../query';

  // What the list is ordered by, and which way. Picking an order uses its
  // natural direction (newest first, A to Z); the direction can then be
  // flipped from the same panel, or by the arrow beside the trigger.
  interface Props {
    keys: readonly SortKey<T>[];
    value: SortState;
    /** Searching: offers "Best match". */
    searching: boolean;
    onChange: (value: SortState) => void;
  }
  let { keys, value, searching, onChange }: Props = $props();

  let open = $state(false);
  const offered = $derived(keys.filter((k) => !k.needsQuery || searching));
  const current = $derived(offered.find((k) => k.id === value.key) ?? offered[0]!);
  const dirLabel = $derived(current.dirs[value.dir === 'asc' ? 0 : 1]);
</script>

<div class="sort">
  <Popover.Root bind:open>
    <Popover.Trigger class="fm-filter-chip" aria-label="Sort by {current.label}, {dirLabel}. Change">
      <Icon name="sort" size={14} />
      <span class="lead">Sort</span>
      <span>{current.label}</span>
      <Icon name="chevron-down" size={12} />
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content class="fm-menu fm-sort-pop" sideOffset={4} align="end">
        <div role="radiogroup" aria-label="Sort by">
          {#each offered as k (k.id)}
            <button
              type="button"
              role="radio"
              aria-checked={k.id === current.id}
              class="fm-menu-item row"
              onclick={() => onChange({ key: k.id, dir: k.id === current.id ? value.dir : k.dir })}
            >
              <span class="fm-menu-check">{#if k.id === current.id}<Icon name="check" size={14} />{/if}</span>
              {k.label}
            </button>
          {/each}
        </div>
        <div class="fm-menu-separator"></div>
        <div role="radiogroup" aria-label="Direction">
          {#each ['asc', 'desc'] as const as d (d)}
            <button
              type="button"
              role="radio"
              aria-checked={value.dir === d}
              class="fm-menu-item row"
              onclick={() => onChange({ key: current.id, dir: d })}
            >
              <span class="fm-menu-check">{#if value.dir === d}<Icon name="check" size={14} />{/if}</span>
              {current.dirs[d === 'asc' ? 0 : 1]}
              <span class="dir-icon"><Icon name={d === 'asc' ? 'sort-asc' : 'sort-desc'} size={14} /></span>
            </button>
          {/each}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
  <button
    type="button"
    class="fm-filter-chip flip"
    aria-label="{dirLabel}. Reverse the order"
    onclick={() => onChange({ key: current.id, dir: value.dir === 'asc' ? 'desc' : 'asc' })}
  >
    <Icon name={value.dir === 'asc' ? 'sort-asc' : 'sort-desc'} size={14} />
  </button>
</div>

<style>
  .sort {
    display: inline-flex;
    flex: none;
  }
  .sort :global(.fm-filter-chip:first-child) {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .flip {
    margin-left: -1px;
    border-top-left-radius: 0 !important;
    border-bottom-left-radius: 0 !important;
    padding: 0 var(--s-2) !important;
  }
  .lead {
    color: var(--muted);
  }
  :global(.fm-sort-pop) {
    width: 220px;
  }
  .row {
    width: 100%;
    text-align: left;
  }
  .row:hover {
    background: var(--hover);
  }
  .dir-icon {
    display: grid;
    margin-left: auto;
    color: var(--muted);
  }
</style>
