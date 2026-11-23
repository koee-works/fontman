<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon, { type IconName } from './Icon.svelte';

  // Inline status for a region: what happened, and what to do next. Errors and
  // warnings are announced; info is polite.
  interface Props {
    tone?: 'info' | 'warning' | 'error' | 'conflict';
    /** `bar` spans a region edge to edge; `box` sits inside a panel. */
    variant?: 'bar' | 'box';
    children: Snippet;
    action?: Snippet;
    onDismiss?: () => void;
  }
  let { tone = 'info', variant = 'bar', children, action, onDismiss }: Props = $props();

  const icons: Record<string, IconName> = {
    info: 'info',
    warning: 'warning',
    error: 'error',
    conflict: 'warning',
  };
</script>

<div class="callout {tone} {variant}" role={tone === 'error' ? 'alert' : 'status'}>
  <Icon name={icons[tone]!} />
  <div class="text">{@render children()}</div>
  {#if action}<div class="action">{@render action()}</div>{/if}
  {#if onDismiss}
    <button class="dismiss" onclick={onDismiss} aria-label="Dismiss">
      <Icon name="x" size={14} />
    </button>
  {/if}
</div>

<style>
  .callout {
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-5);
    border-bottom: 1px solid var(--faint);
    line-height: 1.45;
  }
  .box {
    padding: var(--s-3);
    border-bottom: 0;
    border-radius: var(--r-2);
  }
  .callout > :global(svg) {
    margin-top: 1px;
  }
  .text {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .action {
    flex: none;
    margin: -4px 0;
  }
  .info {
    background: var(--cyan-soft);
  }
  .info > :global(svg) {
    color: var(--cyan);
  }
  .warning {
    background: var(--amber-soft);
  }
  .warning > :global(svg),
  .warning .text {
    color: var(--amber);
  }
  .error {
    background: var(--red-soft);
  }
  .error > :global(svg),
  .error .text {
    color: var(--red);
  }
  .conflict {
    background: var(--magenta-soft);
  }
  .conflict > :global(svg),
  .conflict .text {
    color: var(--magenta);
  }
  .dismiss {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin: -2px -6px 0 0;
    border-radius: var(--r-1);
    color: var(--muted);
  }
  .dismiss:hover {
    background: rgb(0 0 0 / 0.06);
    color: var(--ink);
  }
</style>
