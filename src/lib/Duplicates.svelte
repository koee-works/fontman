<script lang="ts">
  import { onMount } from 'svelte';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import { api, type DuplicateCopy, type DuplicateGroup } from './api';
  import VirtualList from './VirtualList.svelte';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import ConfirmDialog from './ui/ConfirmDialog.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';

  // Files with identical bytes. The app uses one copy of each and ignores the
  // rest; this screen lists them so the extras can go to the Trash.
  interface Props {
    /** Called after files are trashed, so the library can refresh. */
    onChanged: () => void;
  }
  let { onChanged }: Props = $props();

  let groups = $state.raw<DuplicateGroup[] | null>(null);
  let notice = $state<{ tone: 'info' | 'warning' | 'error'; text: string } | null>(null);
  let busy = $state(false);
  let confirmAll = $state(false);

  onMount(load);

  async function load() {
    try {
      groups = await api.duplicateGroups();
    } catch (e) {
      notice = { tone: 'error', text: `Couldn't list duplicate files: ${e}` };
      groups = [];
    }
  }

  const existing = (g: DuplicateGroup) => g.copies.filter((c) => c.exists).length;

  /** Why this copy can't be trashed, or null if it can. */
  function blocked(g: DuplicateGroup, c: DuplicateCopy): string | null {
    if (c.installed) return 'In a system font folder, which the OS manages';
    if (c.active) return 'An active style is registered from this copy';
    if (!c.exists) return 'Already gone from disk';
    if (existing(g) <= 1) return 'The last copy';
    return null;
  }

  /** The copies "keep one" would trash: all but the copy in use, where allowed. */
  const extras = (g: DuplicateGroup) => g.copies.filter((c) => !c.inUse && blocked(g, c) === null);

  const allExtras = $derived(groups?.flatMap((g) => extras(g).map((c) => ({ c, size: g.fileSize }))) ?? []);
  const extraBytes = $derived(allExtras.reduce((n, x) => n + x.size, 0));
  const extraCopies = $derived(groups?.reduce((n, g) => n + g.copies.length - 1, 0) ?? 0);

  const bytes = (n: number) =>
    n >= 1024 ** 3
      ? `${(n / 1024 ** 3).toFixed(1)} GB`
      : n >= 1024 ** 2
        ? `${(n / 1024 ** 2).toFixed(1)} MB`
        : `${Math.max(1, Math.round(n / 1024))} KB`;
  const plural = (n: number, one: string, many = `${one}s`) => `${n.toLocaleString()} ${n === 1 ? one : many}`;

  function split(path: string) {
    const i = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
    return { dir: path.slice(0, i + 1), name: path.slice(i + 1) };
  }

  async function trash(paths: string[]) {
    if (!paths.length || busy) return;
    busy = true;
    const sizeOf = new Map(groups?.flatMap((g) => g.copies.map((c) => [c.path, g.fileSize] as const)));
    try {
      const report = await api.trashDuplicates(paths);
      groups = report.groups;
      const ok = report.results.filter((r) => r.ok);
      const failed = report.results.filter((r) => !r.ok);
      const freed = ok.reduce((n, r) => n + (sizeOf.get(r.path) ?? 0), 0);
      const done = ok.length
        ? `Moved ${plural(ok.length, 'copy', 'copies')} to the Trash, freeing ${bytes(freed)}. You can restore ${ok.length === 1 ? 'it' : 'them'} from the Trash.`
        : '';
      const kept = failed.length
        ? ` ${plural(failed.length, 'copy was', 'copies were')} kept: ${failed[0]!.error}`
        : '';
      notice = { tone: failed.length ? (ok.length ? 'warning' : 'error') : 'info', text: (done + kept).trim() };
      if (ok.length) onChanged();
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      busy = false;
    }
  }

  function reveal(path: string) {
    revealItemInDir(path).catch((e) => (notice = { tone: 'error', text: `Couldn't open the folder: ${e}` }));
  }

  const estimate = (g: DuplicateGroup) => 16 + 58 + g.copies.length * 44 + 12;
</script>

<div class="dupes">
  <header class="toolbar">
    <h1>Duplicate files</h1>
    {#if groups?.length}
      <span class="summary">
        {plural(extraCopies, 'extra copy', 'extra copies')} of {plural(groups.length, 'font file')}{#if allExtras.length},
          {bytes(extraBytes)} you can free{/if}
      </span>
    {/if}
    <span class="spacer"></span>
    {#if allExtras.length}
      <Button size="sm" variant="soft" tone="danger" icon="trash" disabled={busy} onclick={() => (confirmAll = true)}>
        Trash all extra copies…
      </Button>
    {/if}
  </header>

  {#if notice}
    <Callout tone={notice.tone} onDismiss={() => (notice = null)}>{notice.text}</Callout>
  {/if}

  {#if groups === null}
    <p class="empty">Looking for duplicate files…</p>
  {:else if groups.length === 0}
    <div class="empty">
      <span class="empty-icon"><Icon name="duplicates" size={20} /></span>
      <h2>No duplicate files</h2>
      <p>Every font file in your library is unique. Files with identical bytes would be listed here.</p>
    </div>
  {:else}
    <p class="explain">
      Each group below is one font saved more than once, byte for byte. The app previews and
      activates from the copy marked <strong>In use</strong> and ignores the others. Copies go to
      the Trash, so they can be restored. System fonts, and copies an active style is registered
      from, are always kept.
    </p>
    <VirtualList items={groups} key={(g) => g.contentHash} {estimate} label="Duplicate files">
      {#snippet row(g)}
        {@const ex = extras(g)}
        <article class="group" aria-label={g.faces[0] ?? 'Font file'}>
          <div class="group-head">
            <div class="title">
              <h2>{g.faces[0] ?? 'Unreadable font'}</h2>
              <p>
                {#if g.faces.length > 1}{plural(g.faces.length - 1, 'more style')} in this file,
                {/if}{bytes(g.fileSize)} each, {plural(g.copies.length, 'copy', 'copies')}
              </p>
            </div>
            {#if ex.length}
              <Button size="sm" variant="ghost" disabled={busy} onclick={() => trash(ex.map((c) => c.path))}>
                Keep one, trash {plural(ex.length, 'other')}
              </Button>
            {/if}
          </div>
          <ul class="copies">
            {#each g.copies as c (c.path)}
              {@const why = blocked(g, c)}
              {@const p = split(c.path)}
              <li class:gone={!c.exists}>
                <span class="path" aria-label={c.path}><span class="dir">{p.dir}</span><span class="file">{p.name}</span></span>
                {#if c.inUse}<span class="pill on">In use</span>{/if}
                {#if c.installed}<span class="pill"><Icon name="lock" size={12} />System</span>{/if}
                {#if c.active}<span class="pill on"><Icon name="active" size={12} />Active</span>{/if}
                {#if !c.exists}<span class="pill">Missing</span>{/if}
                <IconButton icon="reveal" size="sm" label="Show in folder" disabled={!c.exists} onclick={() => reveal(c.path)} />
                <IconButton
                  icon="trash"
                  size="sm"
                  tone="danger"
                  label={why ? `Can't move to the Trash: ${why.toLowerCase()}` : 'Move this copy to the Trash'}
                  disabled={busy || why !== null}
                  onclick={() => trash([c.path])}
                />
              </li>
            {/each}
          </ul>
        </article>
      {/snippet}
    </VirtualList>
  {/if}
</div>

<ConfirmDialog
  bind:open={confirmAll}
  title="Move {plural(allExtras.length, 'extra copy', 'extra copies')} to the Trash?"
  confirmLabel="Move to Trash"
  onConfirm={() => trash(allExtras.map((x) => x.c.path))}
>
  One copy of each font stays where it is: the one in use. This frees {bytes(extraBytes)}, and the
  copies can be restored from the Trash.
</ConfirmDialog>

<style>
  .dupes {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: 52px;
    flex: none;
    padding: 0 var(--s-4) 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  h1 {
    font-size: var(--text-3);
    font-weight: var(--weight-strong);
    margin: 0;
    white-space: nowrap;
  }
  .summary {
    color: var(--muted);
    font-size: var(--text-1);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .explain {
    margin: 0;
    padding: var(--s-4) var(--s-5) 0;
    max-width: 80ch;
    color: var(--muted);
    line-height: 1.55;
  }
  .explain strong {
    color: var(--ink);
    font-weight: 500;
  }

  .group {
    margin: var(--s-4) var(--s-5) 0;
    padding: var(--s-3) var(--s-2) var(--s-2) var(--s-4);
    border: 1px solid var(--faint);
    border-radius: var(--r-3);
  }
  .group-head {
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
    padding-right: var(--s-1);
    margin-bottom: var(--s-2);
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  .title h2 {
    font-size: var(--text-2);
    font-weight: var(--weight-strong);
    margin: 0;
  }
  .title p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-1);
  }
  .copies {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .copies li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 36px;
    padding: var(--s-1) 0;
    border-top: 1px solid var(--faint);
  }
  .copies li.gone .path {
    text-decoration: line-through;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: var(--text-1);
    overflow-wrap: anywhere;
    line-height: 1.45;
  }
  .dir {
    color: var(--muted);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: 20px;
    padding: 0 var(--s-2);
    border-radius: var(--r-full);
    background: var(--hover);
    color: var(--muted);
    font-size: var(--text-1);
    font-weight: 500;
    white-space: nowrap;
  }
  .pill.on {
    background: var(--cyan-soft);
    color: var(--cyan);
  }

  .empty {
    padding: var(--s-8) var(--s-5);
    color: var(--muted);
    max-width: 52ch;
    margin: 0;
  }
  .empty h2 {
    color: var(--ink);
    font-size: var(--text-4);
    margin: 0 0 var(--s-2);
  }
  .empty p {
    margin: 0;
    line-height: 1.55;
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin-bottom: var(--s-4);
    border-radius: var(--r-full);
    background: var(--cyan-soft);
    color: var(--cyan);
  }
</style>
