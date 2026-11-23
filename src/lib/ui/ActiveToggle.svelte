<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import Icon from './Icon.svelte';

  // Switches fonts on and off, and shows whether they are: an empty circle
  // when off, a half-filled circle when some styles of a family are on, a
  // ticked circle when all are. Checkboxes are for selection only, so this is
  // the one control for activation. Installed fonts show a lock: always on.
  interface Props {
    state: 'none' | 'partial' | 'all' | 'installed';
    /** What it switches, for labels: "Inter" or "Inter Bold". */
    name: string;
    /** Library styles on, and how many there are (families only). */
    on?: number;
    total?: number;
    disabled?: boolean;
    onToggle?: (active: boolean) => void;
  }
  let { state, name, on = 0, total = 1, disabled = false, onToggle }: Props = $props();

  const styles = (n: number) => `${n} ${n === 1 ? 'style' : 'styles'}`;
  const label = $derived(
    state === 'all'
      ? total > 1
        ? `All ${styles(total)} of ${name} active. Deactivate`
        : `${name} is active. Deactivate`
      : state === 'partial'
        ? `${on} of ${styles(total)} of ${name} active. Activate the other ${total - on}`
        : total > 1
          ? `Activate all ${styles(total)} of ${name}`
          : `Activate ${name}`,
  );
</script>

{#if state === 'installed'}
  <span class="fm-active-lock" title="Installed by the system; always on">
    <Icon name="lock" size={14} label="{name} is installed by the system; always on" />
  </span>
{:else}
  <Tooltip.Root>
    <Tooltip.Trigger
      class="fm-active-toggle {state}"
      aria-label={label}
      aria-pressed={state === 'all' ? 'true' : state === 'partial' ? 'mixed' : 'false'}
      {disabled}
      onclick={() => onToggle?.(state !== 'all')}
    >
      <Icon name={state === 'all' ? 'active' : state === 'partial' ? 'active-some' : 'inactive'} size={16} />
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content class="fm-tooltip" sideOffset={6}>{label}</Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
{/if}

<style>
  :global(.fm-active-toggle) {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: var(--r-full);
    color: var(--muted);
    transition: background-color var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }
  :global(.fm-active-toggle.partial),
  :global(.fm-active-toggle.all) {
    color: var(--cyan);
  }
  :global(.fm-active-toggle:not(:disabled):hover) {
    background: var(--hover);
    color: var(--cyan);
  }
  :global(.fm-active-toggle:disabled) {
    opacity: 0.45;
  }
  :global(.fm-active-lock) {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    color: var(--muted);
  }
</style>
