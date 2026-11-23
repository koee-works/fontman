<script lang="ts">
  import { Dialog } from 'bits-ui';

  // Asks for a name. `onSubmit` returns an error message to show under the
  // field (the dialog stays open), or null when done.
  interface Props {
    open: boolean;
    title: string;
    label: string;
    confirmLabel: string;
    value?: string;
    description?: string;
    onSubmit: (value: string) => Promise<string | null>;
  }
  let {
    open = $bindable(),
    title,
    label,
    confirmLabel,
    value = '',
    description,
    onSubmit,
  }: Props = $props();

  let text = $state('');
  let error = $state<string | null>(null);
  let pending = $state(false);
  let input = $state<HTMLInputElement>();

  // Start from the given value each time the dialog opens.
  $effect(() => {
    if (open) {
      text = value;
      error = null;
      pending = false;
    }
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (pending) return;
    if (!text.trim()) {
      error = 'Give it a name.';
      input?.focus();
      return;
    }
    pending = true;
    try {
      error = await onSubmit(text);
      if (error === null) open = false;
      else input?.select();
    } finally {
      pending = false;
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fm-overlay" />
    <Dialog.Content
      class="fm-dialog"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        input?.focus();
        input?.select();
      }}
    >
      <Dialog.Title class="fm-dialog-title">{title}</Dialog.Title>
      {#if description}
        <Dialog.Description class="fm-dialog-body short">{description}</Dialog.Description>
      {/if}
      <form onsubmit={submit}>
        <label class="field">
          <span>{label}</span>
          <input
            bind:this={input}
            bind:value={text}
            type="text"
            maxlength="80"
            spellcheck="false"
            autocomplete="off"
            aria-invalid={error ? 'true' : undefined}
            aria-describedby={error ? 'prompt-error' : undefined}
            oninput={() => (error = null)}
          />
        </label>
        {#if error}<p class="error" id="prompt-error" role="alert">{error}</p>{/if}
        <div class="fm-dialog-actions">
          <Dialog.Close class="fm-dialog-btn cancel" type="button">Cancel</Dialog.Close>
          <button class="fm-dialog-btn accent" type="submit" disabled={pending}>{confirmLabel}</button>
        </div>
      </form>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.fm-dialog-body.short) {
    margin-bottom: var(--s-4);
  }
  .field {
    display: grid;
    gap: var(--s-2);
    margin: var(--s-4) 0 var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  input {
    height: var(--control);
    padding: 0 var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    color: var(--ink);
    font-size: var(--text-2);
  }
  input:focus-visible {
    outline-offset: 0;
    border-color: var(--focus);
  }
  input[aria-invalid='true'] {
    border-color: var(--red);
  }
  .error {
    margin: 0 0 var(--s-2);
    color: var(--red);
    font-size: var(--text-1);
  }
  .fm-dialog-actions {
    margin-top: var(--s-5);
  }
  button:disabled {
    opacity: 0.6;
  }
</style>
