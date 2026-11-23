<script lang="ts">
  import { AlertDialog } from 'bits-ui';
  import type { Snippet } from 'svelte';

  // For actions that can't be undone. The confirm button names the action
  // ("Remove folder"), never "OK", and Cancel is focused first.
  interface Props {
    open: boolean;
    title: string;
    confirmLabel: string;
    tone?: 'danger' | 'accent';
    onConfirm: () => void;
    children: Snippet;
  }
  let { open = $bindable(), title, confirmLabel, tone = 'danger', onConfirm, children }: Props = $props();
</script>

<AlertDialog.Root bind:open>
  <AlertDialog.Portal>
    <AlertDialog.Overlay class="fm-overlay" />
    <AlertDialog.Content class="fm-dialog">
      <AlertDialog.Title class="fm-dialog-title">{title}</AlertDialog.Title>
      <AlertDialog.Description class="fm-dialog-body">
        {@render children()}
      </AlertDialog.Description>
      <div class="fm-dialog-actions">
        <AlertDialog.Cancel class="fm-dialog-btn cancel">Cancel</AlertDialog.Cancel>
        <AlertDialog.Action
          class="fm-dialog-btn {tone}"
          onclick={() => {
            open = false;
            onConfirm();
          }}
        >
          {confirmLabel}
        </AlertDialog.Action>
      </div>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>

<style>
  :global(.fm-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: var(--z-dialog);
    animation: fade-in var(--dur) var(--ease);
  }
  :global(.fm-dialog) {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: min(440px, calc(100vw - 32px));
    padding: var(--s-6);
    background: var(--raised);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-float);
    z-index: calc(var(--z-dialog) + 1);
    animation: float-in var(--dur) var(--ease);
  }
  :global(.fm-dialog-title) {
    font-size: var(--text-4);
    font-weight: var(--weight-strong);
    margin: 0 0 var(--s-2);
  }
  :global(.fm-dialog-body) {
    color: var(--muted);
    line-height: 1.55;
    margin: 0 0 var(--s-6);
  }
  :global(.fm-dialog-actions) {
    display: flex;
    justify-content: flex-end;
    gap: var(--s-2);
  }
  :global(.fm-dialog-btn) {
    height: var(--control);
    padding: 0 var(--s-4);
    border-radius: var(--r-2);
    font-weight: 500;
  }
  :global(.fm-dialog-btn.cancel) {
    border: 1px solid var(--line);
    background: var(--sheet);
  }
  :global(.fm-dialog-btn.cancel:hover) {
    background: var(--hover);
  }
  :global(.fm-dialog-btn.danger) {
    background: var(--red-solid);
    color: var(--red-on);
    font-weight: var(--weight-strong);
  }
  :global(.fm-dialog-btn.accent) {
    background: var(--cyan-solid);
    color: var(--cyan-on);
    font-weight: var(--weight-strong);
  }
</style>
