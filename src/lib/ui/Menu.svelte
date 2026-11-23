<script lang="ts" module>
  import type { IconName } from './Icon.svelte';

  export interface MenuItem {
    label: string;
    icon?: IconName;
    tone?: 'neutral' | 'danger';
    disabled?: boolean;
    onSelect: () => void;
  }
</script>

<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import Icon from './Icon.svelte';

  // A "more actions" menu behind a ⋯ button, or behind a labelled button
  // when `text` is given. Destructive items go last, after a separator, in red.
  interface Props {
    label: string;
    items: MenuItem[];
    /** Visible trigger text; without it the trigger is ⋯. */
    text?: string;
    icon?: IconName;
  }
  let { label, items, text, icon }: Props = $props();

  const safe = $derived(items.filter((i) => i.tone !== 'danger'));
  const danger = $derived(items.filter((i) => i.tone === 'danger'));
</script>

<DropdownMenu.Root>
  {#if text}
    <DropdownMenu.Trigger class="fm-menu-btn" aria-label={label}>
      {#if icon}<Icon name={icon} size={14} />{/if}
      {text}
      <Icon name="chevron-down" size={12} />
    </DropdownMenu.Trigger>
  {:else}
    <DropdownMenu.Trigger class="fm-icon-btn sm" aria-label={label}>
      <Icon name="more" size={14} />
    </DropdownMenu.Trigger>
  {/if}
  <DropdownMenu.Portal>
    <DropdownMenu.Content class="fm-menu" sideOffset={4} align="end">
      {#each safe as item (item.label)}
        <DropdownMenu.Item class="fm-menu-item" disabled={item.disabled} onSelect={item.onSelect}>
          <span class="fm-menu-check">{#if item.icon}<Icon name={item.icon} size={14} />{/if}</span>
          {item.label}
        </DropdownMenu.Item>
      {/each}
      {#if danger.length}
        {#if safe.length}<DropdownMenu.Separator class="fm-menu-separator" />{/if}
        {#each danger as item (item.label)}
          <DropdownMenu.Item class="fm-menu-item danger" disabled={item.disabled} onSelect={item.onSelect}>
            <span class="fm-menu-check">{#if item.icon}<Icon name={item.icon} size={14} />{/if}</span>
            {item.label}
          </DropdownMenu.Item>
        {/each}
      {/if}
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  /* A labelled menu trigger. Matches Button soft/neutral/sm, which can't be
     used as a Bits trigger. Shared by every menu with a text trigger. */
  :global(.fm-menu-btn) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: var(--control-sm);
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    background: var(--hover);
    color: var(--ink);
    font-size: var(--text-1);
    font-weight: 500;
    white-space: nowrap;
    transition: background-color var(--dur-fast) var(--ease);
  }
  :global(.fm-menu-btn:hover),
  :global(.fm-menu-btn[data-state='open']) {
    background: var(--pressed);
  }
  :global(.fm-menu-heading) {
    padding: var(--s-1) var(--s-2) var(--s-1) calc(var(--s-2) + 16px + var(--s-2));
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--muted);
  }

  /* Shared by Select and Menu: one look for every floating list. */
  :global(.fm-menu) {
    min-width: 180px;
    padding: var(--s-1);
    background: var(--raised);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-float);
    z-index: var(--z-float);
    outline: none;
    animation: float-in var(--dur) var(--ease);
    transform-origin: var(--bits-dropdown-menu-content-transform-origin, var(--bits-context-menu-content-transform-origin, var(--bits-select-content-transform-origin, top)));
  }
  :global(.fm-menu-item) {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 28px;
    padding: 0 var(--s-3) 0 var(--s-2);
    border-radius: var(--r-2);
    cursor: default;
    outline: none;
    user-select: none;
  }
  /* The highlight shows the active item; no focus outline on top of it. */
  :global(.fm-menu-item:focus-visible) {
    outline: none;
  }
  :global(.fm-menu-item[data-highlighted]) {
    background: var(--hover);
  }
  :global(.fm-menu-item[data-disabled]) {
    opacity: 0.45;
  }
  :global(.fm-menu-item.danger) {
    color: var(--red);
  }
  :global(.fm-menu-item.danger[data-highlighted]) {
    background: var(--red-soft);
  }
  :global(.fm-menu-check) {
    width: 16px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  :global(.fm-menu-item.danger .fm-menu-check) {
    color: inherit;
  }
  :global(.fm-menu-separator) {
    height: 1px;
    margin: var(--s-1) calc(var(--s-1) * -1);
    background: var(--faint);
  }
</style>
