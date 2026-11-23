<script lang="ts">
  import { Select } from 'bits-ui';
  import Icon from './Icon.svelte';

  interface Props {
    items: { value: string; label: string }[];
    value: string;
    label: string;
    onValueChange?: (value: string) => void;
    size?: 'sm' | 'md';
  }
  let { items, value = $bindable(), label, onValueChange, size = 'md' }: Props = $props();

  const selected = $derived(items.find((i) => i.value === value)?.label ?? label);
</script>

<Select.Root type="single" bind:value {items} {onValueChange}>
  <Select.Trigger class="fm-select-trigger {size}" aria-label={label}>
    <span>{selected}</span>
    <Icon name="chevron-down" size={14} />
  </Select.Trigger>
  <Select.Portal>
    <Select.Content class="fm-menu" sideOffset={4} align="start">
      <Select.Viewport>
        {#each items as item (item.value)}
          <Select.Item value={item.value} label={item.label} class="fm-menu-item">
            {#snippet children({ selected })}
              <span class="fm-menu-check">{#if selected}<Icon name="check" size={14} />{/if}</span>
              {item.label}
            {/snippet}
          </Select.Item>
        {/each}
      </Select.Viewport>
    </Select.Content>
  </Select.Portal>
</Select.Root>

<style>
  :global(.fm-select-trigger) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: var(--control);
    padding: 0 var(--s-2) 0 var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    white-space: nowrap;
  }
  :global(.fm-select-trigger.sm) {
    height: var(--control-sm);
    font-size: var(--text-1);
    padding: 0 var(--s-1) 0 var(--s-2);
  }
  :global(.fm-select-trigger:hover) {
    background: var(--hover);
  }
  :global(.fm-select-trigger svg) {
    color: var(--muted);
  }
</style>
