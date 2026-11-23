<script lang="ts">
  import { Tabs } from 'bits-ui';
  import type { Snippet } from 'svelte';

  interface Props {
    value: string;
    tabs: { value: string; label: string }[];
    label: string;
    children: Snippet;
  }
  let { value = $bindable(), tabs, label, children }: Props = $props();
</script>

<Tabs.Root bind:value class="fm-tabs">
  <Tabs.List class="fm-tabs-list" aria-label={label}>
    {#each tabs as t (t.value)}
      <Tabs.Trigger value={t.value} class="fm-tab">{t.label}</Tabs.Trigger>
    {/each}
  </Tabs.List>
  {@render children()}
</Tabs.Root>

<style>
  :global(.fm-tabs) {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  :global(.fm-tabs-list) {
    display: flex;
    gap: var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  :global(.fm-tab) {
    position: relative;
    padding: var(--s-2) 0;
    color: var(--muted);
  }
  :global(.fm-tab:hover) {
    color: var(--ink);
  }
  :global(.fm-tab[data-state='active']) {
    color: var(--ink);
    font-weight: 500;
  }
  :global(.fm-tab[data-state='active'])::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    border-radius: 2px;
    background: var(--ink);
  }
  /* Bits hides inactive panels with the hidden attribute; a display rule
     here would override it, so only visible panels get flex. */
  :global(.fm-tab-panel:not([hidden])) {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
    outline: none;
    /* Each panel eases in when its tab is chosen (Motion setting). */
    animation: tab-in var(--dur) var(--ease);
  }
</style>
