<script lang="ts" generics="V extends string">
  import { ToggleGroup } from 'bits-ui';
  import Icon, { type IconName } from './Icon.svelte';

  // A small set of mutually exclusive choices shown all at once. Always has a
  // value: clicking the selected option does not clear it.
  interface Props {
    items: { value: V; label: string; icon?: IconName }[];
    value: V;
    label: string;
    /** Show only icons; each item's label becomes its accessible name and tooltip. */
    iconOnly?: boolean;
    onValueChange: (value: V) => void;
  }
  let { items, value, label, iconOnly = false, onValueChange }: Props = $props();
</script>

<ToggleGroup.Root
  type="single"
  {value}
  onValueChange={(v) => {
    if (v) onValueChange(v as V);
  }}
  class="fm-segmented"
  aria-label={label}
>
  {#each items as item (item.value)}
    <ToggleGroup.Item
      value={item.value}
      class="fm-segment {iconOnly ? 'icon-only' : ''}"
      aria-label={iconOnly ? item.label : undefined}
      title={iconOnly ? item.label : undefined}
    >
      {#if item.icon}<Icon name={item.icon} size={14} />{/if}
      {#if !iconOnly}{item.label}{/if}
    </ToggleGroup.Item>
  {/each}
</ToggleGroup.Root>

<style>
  :global(.fm-segmented) {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    background: var(--hover);
    border-radius: var(--r-2);
  }
  :global(.fm-segment) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: calc(var(--control) - 4px);
    padding: 0 var(--s-3);
    border-radius: calc(var(--r-2) - 2px);
    color: var(--muted);
    transition: background-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  :global(.fm-segment.icon-only) {
    width: calc(var(--control) - 4px);
    padding: 0;
    justify-content: center;
  }
  :global(.fm-segment:hover) {
    color: var(--ink);
  }
  :global(.fm-segment[data-state='on']) {
    background: var(--sheet);
    color: var(--ink);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.12);
  }
</style>
