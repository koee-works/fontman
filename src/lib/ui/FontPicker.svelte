<script lang="ts">
  import { Combobox } from 'bits-ui';
  import type { FamilySummary } from '../api';
  import { specimen } from '../fonts';
  import Icon from './Icon.svelte';

  // Type to search the whole library; matches preview in their own typeface.
  // Capped so a library of thousands of families stays instant to filter.
  const LIMIT = 60;

  interface Props {
    families: FamilySummary[];
    /** Selected family name. */
    value: string;
    label: string;
    onPick: (family: FamilySummary) => void;
  }
  let { families, value, label, onPick }: Props = $props();

  let search = $state('');
  let open = $state(false);

  const matches = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const hits = q ? families.filter((f) => f.name.toLowerCase().includes(q)) : families;
    return hits.slice(0, LIMIT);
  });
  const more = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const total = q ? families.filter((f) => f.name.toLowerCase().includes(q)).length : families.length;
    return Math.max(0, total - LIMIT);
  });
</script>

<Combobox.Root
  type="single"
  {value}
  bind:open
  items={matches.map((f) => ({ value: f.name, label: f.name }))}
  onOpenChange={(o) => {
    if (o) search = '';
  }}
  onValueChange={(name) => {
    const fam = families.find((f) => f.name === name);
    if (fam) onPick(fam);
  }}
>
  <div class="fm-combo">
    <Combobox.Input
      class="fm-combo-input"
      defaultValue={value}
      aria-label={label}
      placeholder="Search fonts"
      oninput={(e) => (search = e.currentTarget.value)}
    />
    <Combobox.Trigger class="fm-combo-trigger" aria-label="Show fonts">
      <Icon name="chevron-down" size={14} />
    </Combobox.Trigger>
  </div>
  <Combobox.Portal>
    <Combobox.Content class="fm-menu fm-combo-content" sideOffset={4} align="start">
      <Combobox.Viewport>
        {#each matches as f (f.name)}
          <Combobox.Item value={f.name} label={f.name} class="fm-menu-item fm-combo-item">
            {#snippet children({ selected })}
              <span class="fm-menu-check">{#if selected}<Icon name="check" size={14} />{/if}</span>
              <span class="fm-combo-preview" use:specimen={f.representative}>{f.name}</span>
            {/snippet}
          </Combobox.Item>
        {:else}
          <p class="fm-combo-empty">No font matches “{search}”.</p>
        {/each}
        {#if more}
          <p class="fm-combo-empty">{more.toLocaleString()} more; keep typing to narrow it down.</p>
        {/if}
      </Combobox.Viewport>
    </Combobox.Content>
  </Combobox.Portal>
</Combobox.Root>

<style>
  .fm-combo {
    display: flex;
    align-items: center;
    height: var(--control);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
  }
  .fm-combo:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
  }
  :global(.fm-combo-input) {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0 var(--s-3);
    border: 0;
    background: none;
    outline: none;
  }
  :global(.fm-combo-trigger) {
    display: grid;
    place-items: center;
    width: 28px;
    height: 100%;
    color: var(--muted);
  }
  :global(.fm-combo-content) {
    width: var(--bits-combobox-anchor-width, 260px);
    min-width: 240px;
    max-height: min(360px, var(--bits-combobox-content-available-height, 360px));
    overflow-y: auto;
  }
  :global(.fm-combo-item) {
    height: 34px;
  }
  :global(.fm-combo-preview) {
    flex: 1;
    min-width: 0;
    font-size: 17px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-synthesis: none;
  }
  :global(.fm-combo-empty) {
    margin: 0;
    padding: var(--s-2) var(--s-3);
    color: var(--muted);
    font-size: var(--text-1);
  }
</style>
