<script lang="ts">
  import { Checkbox } from 'bits-ui';
  import Icon from './Icon.svelte';

  // Tri-state: `indeterminate` shows a dash (some styles of a family active).
  // Controlled: the parent decides the state, so a toggle that fails on the
  // backend never leaves the box showing something untrue.
  interface Props {
    checked: boolean;
    indeterminate?: boolean;
    disabled?: boolean;
    label: string;
    onCheckedChange: (checked: boolean) => void;
  }
  let { checked, indeterminate = false, disabled = false, label, onCheckedChange }: Props = $props();
</script>

<Checkbox.Root
  class="fm-checkbox"
  {checked}
  {indeterminate}
  {disabled}
  aria-label={label}
  onCheckedChange={(v) => onCheckedChange(indeterminate ? true : v)}
>
  {#snippet children({ checked, indeterminate })}
    {#if indeterminate}
      <Icon name="minus" size={12} />
    {:else if checked}
      <Icon name="check" size={12} />
    {/if}
  {/snippet}
</Checkbox.Root>

<style>
  :global(.fm-checkbox) {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: var(--r-1);
    border: 1.5px solid var(--line);
    background: var(--sheet);
    color: var(--cyan-on);
    transition: background-color var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }
  :global(.fm-checkbox:not(:disabled):hover) {
    border-color: var(--muted);
  }
  :global(.fm-checkbox[data-state='checked']),
  :global(.fm-checkbox[data-state='indeterminate']) {
    background: var(--cyan-solid);
    border-color: var(--cyan-solid);
  }
  :global(.fm-checkbox:disabled) {
    opacity: 0.5;
    cursor: default;
  }
  :global(.fm-checkbox svg) {
    stroke-width: 2.25;
  }
</style>
