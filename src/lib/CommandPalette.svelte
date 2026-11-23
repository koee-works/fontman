<script lang="ts">
  import { Dialog } from 'bits-ui';
  import { highlight, matchCommand, type Command, type Match } from './palette';
  import Icon from './ui/Icon.svelte';

  // Every action in the app, one keystroke away (Ctrl/⌘+Shift+P), in the
  // manner of Zed's command palette. Type to fuzzy-match; arrows move; Enter
  // runs. Recently run commands come first when nothing is typed.
  interface Props {
    open: boolean;
    commands: Command[];
    /** Extra results for a query, after commands: families, say. */
    extra?: (query: string) => Command[];
  }
  let { open = $bindable(), commands, extra }: Props = $props();

  const RECENT_KEY = 'palette-recent';
  const loadRecent = (): string[] => {
    try {
      return JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]');
    } catch {
      return [];
    }
  };

  let query = $state('');
  let active = $state(0);
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLUListElement>();
  let recent = $state<string[]>(loadRecent());

  $effect(() => {
    if (open) {
      query = '';
      active = 0;
      recent = loadRecent();
    }
  });

  type Row = { cmd: Command; match: Match | null; recent: boolean };
  /** One row per id, whatever the caller passed: a repeat would break the list. */
  const unique = (rs: Row[]) => {
    const seen = new Set<string>();
    return rs.filter((r) => !seen.has(r.cmd.id) && (seen.add(r.cmd.id), true));
  };
  const rows = $derived.by((): Row[] => unique(rowsFor(query.trim())));
  function rowsFor(q: string): Row[] {
    if (!q) {
      const byId = new Map(commands.map((c) => [c.id, c]));
      const first = recent.map((id) => byId.get(id)).filter((c): c is Command => !!c);
      const seen = new Set(first.map((c) => c.id));
      return [
        ...first.map((cmd) => ({ cmd, match: null, recent: true })),
        ...commands.filter((c) => !seen.has(c.id)).map((cmd) => ({ cmd, match: null, recent: false })),
      ];
    }
    const scored = commands
      .map((cmd) => ({ cmd, match: matchCommand(q, cmd), recent: false }))
      .filter((r) => r.match)
      .sort((a, b) => b.match!.score - a.match!.score);
    const more = (extra?.(q) ?? []).map((cmd) => ({ cmd, match: matchCommand(q, cmd), recent: false }));
    return [...scored, ...more];
  }

  $effect(() => {
    if (active >= rows.length) active = Math.max(0, rows.length - 1);
  });
  // Keep the highlighted row in view.
  $effect(() => {
    const el = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  });

  function run(i: number) {
    const row = rows[i];
    if (!row) return;
    open = false;
    try {
      const next = [row.cmd.id, ...recent.filter((id) => id !== row.cmd.id)].slice(0, 8);
      localStorage.setItem(RECENT_KEY, JSON.stringify(next));
    } catch {
      /* not remembered */
    }
    // After the dialog has closed, so focus can go where the command sends it.
    setTimeout(() => row.cmd.run(), 0);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(rows.length - 1, active + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(0, active - 1);
    } else if (e.key === 'PageDown') {
      e.preventDefault();
      active = Math.min(rows.length - 1, active + 8);
    } else if (e.key === 'PageUp') {
      e.preventDefault();
      active = Math.max(0, active - 8);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(active);
    }
  }
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="fm-palette-overlay" />
    <Dialog.Content
      class="fm-palette"
      onOpenAutoFocus={(e) => {
        e.preventDefault();
        input?.focus();
      }}
    >
      <Dialog.Title class="visually-hidden">Command palette</Dialog.Title>
      <div class="field">
        <Icon name="search" />
        <input
          bind:this={input}
          bind:value={query}
          placeholder="Run a command, or go to a family, collection or tag"
          spellcheck="false"
          autocomplete="off"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-list"
          aria-activedescendant={rows.length ? `palette-${active}` : undefined}
          aria-label="Command"
          onkeydown={onKeydown}
          oninput={() => (active = 0)}
        />
      </div>
      <ul class="list" id="palette-list" role="listbox" aria-label="Commands" bind:this={list}>
        <!-- Combobox pattern: the keyboard stays in the field, which moves the
             highlight; rows take pointer clicks only. -->
        {#each rows as row, i (row.cmd.id)}
          {@const parts = highlight(row.cmd.title, row.match?.indices ?? [])}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            id="palette-{i}"
            data-index={i}
            role="option"
            aria-selected={i === active}
            class:active={i === active}
            onpointermove={() => (active = i)}
            onclick={() => run(i)}
          >
            <span class="group">{row.cmd.group}</span>
            <span class="title">
              {#each parts as p, j (j)}{#if p.hit}<mark>{p.text}</mark>{:else}{p.text}{/if}{/each}
            </span>
            {#if row.recent && i === 0}<span class="recent">Recent</span>{/if}
            {#if row.cmd.shortcut}<kbd>{row.cmd.shortcut}</kbd>{/if}
          </li>
        {:else}
          <li class="none">No command matches “{query}”.</li>
        {/each}
      </ul>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  /* Lighter than a modal's overlay: the palette is a quick, passing layer. */
  :global(.fm-palette-overlay) {
    position: fixed;
    inset: 0;
    z-index: var(--z-palette);
    background: rgb(0 0 0 / 0.12);
  }
  :global(.fm-palette) {
    position: fixed;
    top: 12vh;
    left: 50%;
    translate: -50% 0;
    z-index: calc(var(--z-palette) + 1);
    width: min(600px, calc(100vw - 32px));
    display: flex;
    flex-direction: column;
    max-height: min(480px, 76vh);
    background: var(--raised);
    border-radius: var(--r-3);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    outline: none;
  }
  /* Opens with a short drop into place and closes the same way back. Bits
     keeps the palette mounted until the closing animation ends. */
  :global(.fm-palette[data-state='open']) {
    animation: palette-in var(--dur) var(--ease);
  }
  :global(.fm-palette[data-state='closed']) {
    animation: palette-out var(--dur) var(--ease) forwards;
  }
  :global(.fm-palette-overlay[data-state='open']) {
    animation: fade-in var(--dur) var(--ease);
  }
  :global(.fm-palette-overlay[data-state='closed']) {
    animation: fade-out var(--dur) var(--ease) forwards;
  }
  @keyframes -global-palette-in {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
  }
  @keyframes -global-palette-out {
    to {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
  }
  @keyframes -global-fade-out {
    to {
      opacity: 0;
    }
  }
  :global(.fm-palette:focus-visible) {
    outline: none;
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 0 var(--s-3);
    border-bottom: 1px solid var(--faint);
    color: var(--muted);
  }
  input {
    flex: 1;
    height: 44px;
    border: 0;
    background: none;
    color: var(--ink);
    font-size: var(--text-3);
    outline: none;
  }
  input::placeholder {
    color: var(--muted);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: var(--s-1);
    overflow-y: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 30px;
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    cursor: default;
  }
  li.active {
    background: var(--selected);
  }
  .group {
    flex: none;
    color: var(--muted);
  }
  .group::after {
    content: ':';
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  mark {
    background: none;
    color: var(--cyan);
    font-weight: var(--weight-strong);
  }
  .recent {
    font-size: var(--text-1);
    color: var(--muted);
  }
  kbd {
    flex: none;
    padding: 1px var(--s-1);
    border: 1px solid var(--line);
    border-radius: var(--r-1);
    font: var(--text-1) var(--ui);
    color: var(--muted);
  }
  .none {
    color: var(--muted);
  }
</style>
