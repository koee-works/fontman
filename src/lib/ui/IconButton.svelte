<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import Icon, { type IconName } from './Icon.svelte';

  // An icon-only button always has a name: it's the accessible label and the
  // tooltip, so the meaning is never only in the picture.
  interface Props {
    icon: IconName;
    label: string;
    onclick?: (e: MouseEvent) => void;
    tone?: 'neutral' | 'accent' | 'danger';
    size?: 'sm' | 'md';
    disabled?: boolean;
    pressed?: boolean;
  }
  let { icon, label, onclick, tone = 'neutral', size = 'md', disabled, pressed }: Props = $props();
</script>

<Tooltip.Root>
  <Tooltip.Trigger
    class="fm-icon-btn {tone} {size}"
    aria-label={label}
    aria-pressed={pressed}
    {disabled}
    {onclick}
  >
    <Icon name={icon} size={size === 'sm' ? 14 : 16} />
  </Tooltip.Trigger>
  <Tooltip.Portal>
    <Tooltip.Content class="fm-tooltip" sideOffset={6}>{label}</Tooltip.Content>
  </Tooltip.Portal>
</Tooltip.Root>

<style>
  :global(.fm-icon-btn) {
    display: inline-grid;
    place-items: center;
    width: var(--control);
    height: var(--control);
    border-radius: var(--r-2);
    color: var(--muted);
    flex: none;
    transition: background-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  :global(.fm-icon-btn.sm) {
    width: var(--control-sm);
    height: var(--control-sm);
  }
  :global(.fm-icon-btn:not(:disabled):hover) {
    background: var(--hover);
    color: var(--ink);
  }
  :global(.fm-icon-btn:not(:disabled):active),
  :global(.fm-icon-btn[aria-pressed='true']) {
    background: var(--pressed);
    color: var(--ink);
  }
  :global(.fm-icon-btn.accent) { color: var(--cyan); }
  :global(.fm-icon-btn.danger:not(:disabled):hover) { color: var(--red); }
  :global(.fm-icon-btn:disabled) { opacity: 0.45; }

  :global(.fm-tooltip) {
    background: var(--ink);
    color: var(--sheet);
    font-size: var(--text-1);
    padding: var(--s-1) var(--s-2);
    border-radius: var(--r-1);
    max-width: 280px;
    z-index: var(--z-tooltip);
    animation: fade-in var(--dur-fast) var(--ease);
  }
</style>
