<script lang="ts">
  import { Dialog, Tooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import Icon from './ui/Icon.svelte';

  // Settings float over the app in their own window, so the library stays
  // in view behind them and closing returns exactly where you were. Dialogs
  // opened from Settings (confirmations, naming) stack above this one.
  interface Props {
    open: boolean;
    children: Snippet;
  }
  let { open = $bindable(), children }: Props = $props();
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fm-settings-overlay" />
    <Dialog.Content
      class="fm-settings-window"
      onOpenAutoFocus={(e) => {
        // Start in the settings search, ready to type.
        const search = document.querySelector<HTMLInputElement>('.fm-settings-window .pages input');
        if (search) {
          e.preventDefault();
          search.focus();
        }
      }}
    >
      <header class="fm-settings-head">
        <Dialog.Title class="fm-settings-title">Settings</Dialog.Title>
        <Dialog.Close class="fm-icon-btn" aria-label="Close settings">
          <Icon name="x" />
        </Dialog.Close>
      </header>
      <div class="fm-settings-body">
        <!-- Portalled outside the app's provider, so it brings its own. -->
        <Tooltip.Provider delayDuration={500}>
          {@render children()}
        </Tooltip.Provider>
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  /* Below the app's other dialogs (60/61), so a confirmation opened from
     Settings sits on top. */
  :global(.fm-settings-overlay) {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    z-index: var(--z-window);
    animation: fade-in var(--dur) var(--ease);
  }
  :global(.fm-settings-window) {
    position: fixed;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    z-index: calc(var(--z-window) + 1);
    display: flex;
    flex-direction: column;
    width: min(900px, calc(100vw - 48px));
    height: min(680px, calc(100vh - 48px));
    background: var(--sheet);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    outline: none;
    animation: float-in var(--dur) var(--ease);
  }
  /* The window itself takes focus when opened by pointer; its controls show
     focus, the window doesn't. More specific than the global :focus-visible. */
  :global(.fm-settings-window:focus-visible) {
    outline: none;
  }
  .fm-settings-head {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    flex: none;
    height: 52px;
    padding: 0 var(--s-3) 0 var(--s-6);
    border-bottom: 1px solid var(--faint);
  }
  :global(.fm-settings-title) {
    flex: 1;
    margin: 0;
    font-size: var(--text-4);
    font-weight: var(--weight-strong);
  }
  .fm-settings-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
