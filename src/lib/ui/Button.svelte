<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon, { type IconName } from './Icon.svelte';

  // solid   the one primary action in a region
  // soft    secondary actions that still deserve weight
  // outline neutral actions in a row of choices
  // ghost   low-emphasis actions inside lists and toolbars
  interface Props extends HTMLButtonAttributes {
    variant?: 'solid' | 'soft' | 'outline' | 'ghost';
    tone?: 'accent' | 'neutral' | 'danger';
    size?: 'sm' | 'md';
    icon?: IconName;
    children: Snippet;
  }

  let {
    variant = 'outline',
    tone = 'neutral',
    size = 'md',
    icon,
    children,
    type = 'button',
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<button {type} class="btn {variant} {tone} {size} {className}" {...rest}>
  {#if icon}<Icon name={icon} size={size === 'sm' ? 14 : 16} />{/if}
  <span>{@render children()}</span>
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    height: var(--control);
    padding: 0 var(--s-3);
    border-radius: var(--r-2);
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .sm {
    height: var(--control-sm);
    padding: 0 var(--s-2);
    font-size: var(--text-1);
    gap: var(--s-1);
  }
  .btn:disabled {
    opacity: 0.5;
  }

  .solid.accent { background: var(--cyan-solid); color: var(--cyan-on); font-weight: var(--weight-strong); }
  .solid.danger { background: var(--red-solid); color: var(--red-on); font-weight: var(--weight-strong); }
  .solid.neutral { background: var(--ink); color: var(--sheet); }
  .solid:not(:disabled):hover { filter: brightness(1.08); }
  .solid:not(:disabled):active { filter: brightness(0.95); }

  .soft.accent { background: var(--cyan-soft); color: var(--cyan); }
  .soft.danger { background: var(--red-soft); color: var(--red); }
  .soft.neutral { background: var(--hover); color: var(--ink); }
  .soft:not(:disabled):hover { background: var(--pressed); }
  .soft.accent:not(:disabled):hover { background: var(--selected); }

  .outline {
    background: var(--sheet);
    border: 1px solid var(--line);
  }
  .outline.accent { color: var(--cyan); }
  .outline.danger { color: var(--red); }
  .outline:not(:disabled):hover { background: var(--hover); }
  .outline:not(:disabled):active { background: var(--pressed); }

  .ghost.accent { color: var(--cyan); }
  .ghost.danger { color: var(--red); }
  .ghost { padding: 0 var(--s-2); }
  .ghost:not(:disabled):hover { background: var(--hover); }
  .ghost:not(:disabled):active { background: var(--pressed); }
</style>
