<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    api,
    type CatalogView,
    type DownloadProgress,
    type GoogleFamilySummary,
  } from './api';
  import { googlePreviewId, setFaceSizes, specimen } from './fonts';
  import { ago } from './time';
  import VirtualList from './VirtualList.svelte';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import SearchField from './ui/SearchField.svelte';
  import Select from './ui/Select.svelte';

  interface Props {
    keySaved: boolean;
    onOpenSettings: () => void;
    /** Called after downloads so the library list picks up the new fonts. */
    onDownloaded: () => void;
  }
  let { keySaved, onOpenSettings, onDownloaded }: Props = $props();

  const categories = [
    { value: 'all', label: 'All categories' },
    { value: 'sans-serif', label: 'Sans serif' },
    { value: 'serif', label: 'Serif' },
    { value: 'display', label: 'Display' },
    { value: 'handwriting', label: 'Handwriting' },
    { value: 'monospace', label: 'Monospace' },
  ];

  let catalog = $state.raw<CatalogView | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state('');
  let category = $state('all');
  let working = $state.raw(new Set<string>());
  let progress = $state<DownloadProgress | null>(null);
  let notice = $state<{ tone: 'info' | 'error'; text: string } | null>(null);

  const shown = $derived.by(() => {
    if (!catalog) return [];
    const q = query.trim().toLowerCase();
    return catalog.families.filter(
      (f) =>
        (!q || f.family.toLowerCase().includes(q)) &&
        (category === 'all' || f.category === category),
    );
  });
  const updates = $derived(catalog?.families.filter((f) => f.status === 'update') ?? []);

  async function load(refresh = false) {
    if (!keySaved) return;
    loading = true;
    error = null;
    try {
      const view = await api.googleCatalog(refresh);
      // Name-only preview files are a few KB; tell the loader so its memory
      // budget doesn't assume the default.
      setFaceSizes(view.families.map((f) => ({ id: googlePreviewId(f.family), bytes: 8192 })));
      catalog = view;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function download(families: string[]) {
    if (!families.length) return;
    const isUpdate = families.every(
      (name) => catalog?.families.find((f) => f.family === name)?.status === 'update',
    );
    const verb = isUpdate ? 'Updated' : 'Downloaded';
    working = new Set([...working, ...families]);
    notice = null;
    try {
      const results = await api.googleDownload(families);
      const ok = results.filter((r) => r.ok).map((r) => r.family);
      const failed = results.filter((r) => !r.ok);
      await load();
      onDownloaded();
      if (failed.length) {
        notice = {
          tone: 'error',
          text: `${failed[0]!.family} couldn't be ${isUpdate ? 'updated' : 'downloaded'}: ${failed[0]!.error}${failed.length > 1 ? ` (and ${failed.length - 1} more)` : ''}`,
        };
      } else if (ok.length) {
        notice = {
          tone: 'info',
          text: isUpdate
            ? `${verb} ${ok.length === 1 ? ok[0] : `${ok.length} families`}.`
            : ok.length === 1
              ? `${verb} ${ok[0]} to your library. Activate it from My library.`
              : `${verb} ${ok.length} families to your library. Activate them from My library.`,
        };
      }
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      const next = new Set(working);
      for (const f of families) next.delete(f);
      working = next;
      progress = null;
    }
  }

  function meta(f: GoogleFamilySummary): string {
    const parts = [f.styles === 1 ? '1 style' : `${f.styles} styles`];
    if (f.variable) parts.push('variable');
    parts.push(categories.find((c) => c.value === f.category)?.label.toLowerCase() ?? f.category);
    return parts.join(', ');
  }

  $effect(() => {
    if (keySaved && !catalog) load();
  });

  onMount(() => {
    const unlisten = listen<DownloadProgress>('google-download', (e) => (progress = e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<div class="google">
  {#if !keySaved}
    <div class="empty">
      <span class="empty-icon"><Icon name="globe" size={20} /></span>
      <h2>Connect Google Fonts</h2>
      <p>
        Browse more than a thousand open-source families and download any of them into your
        library. Google asks for a free API key; Settings explains how to get one.
      </p>
      <Button variant="solid" tone="accent" onclick={onOpenSettings}>Open Settings</Button>
    </div>
  {:else}
    <div class="toolbar">
      <div class="search">
        <SearchField bind:value={query} size="lg" placeholder="Search Google Fonts" label="Search Google Fonts" />
      </div>
      <Select items={categories} bind:value={category} label="Category" />
      <span class="updated" aria-live="polite">
        {#if loading}Loading…{:else if catalog}Updated {ago(catalog.fetchedAt)}{/if}
      </span>
      <IconButton icon="refresh" label="Refresh the font list" disabled={loading} onclick={() => load(true)} />
    </div>

    {#if catalog?.stale}
      <Callout tone="warning">{catalog.stale}</Callout>
    {/if}
    {#if notice}
      <Callout tone={notice.tone} onDismiss={() => (notice = null)}>{notice.text}</Callout>
    {/if}
    {#if progress && progress.total > 1 && progress.done < progress.total}
      <Callout>Downloading {progress.family}, {progress.done + 1} of {progress.total}</Callout>
    {/if}
    {#if updates.length}
      <Callout>
        {updates.length === 1
          ? `An update is available for ${updates[0]!.family}.`
          : `Updates are available for ${updates.length} downloaded families.`}
        {#snippet action()}
          <Button size="sm" variant="soft" tone="accent" onclick={() => download(updates.map((u) => u.family))}>
            {updates.length === 1 ? 'Update' : 'Update all'}
          </Button>
        {/snippet}
      </Callout>
    {/if}

    {#if error}
      <div class="empty">
        <Callout tone="error" variant="box">{error}</Callout>
        <div class="row">
          <Button variant="solid" tone="accent" onclick={() => load(true)}>Try again</Button>
          <Button onclick={onOpenSettings}>Open Settings</Button>
        </div>
      </div>
    {:else if catalog && shown.length}
      <p class="hint">
        {shown.length.toLocaleString()} {shown.length === 1 ? 'family' : 'families'}. Previews show
        each family's name; download one to try your own text.
      </p>
      <VirtualList
        items={shown}
        key={(f) => f.family}
        estimate={() => 88}
        resetScroll={`${query}|${category}`}
        label="Google Fonts families"
      >
        {#snippet row(f)}
          <article class="family">
            <div class="text">
              <p class="specimen" use:specimen={googlePreviewId(f.family)}>{f.family}</p>
              <p class="meta">{meta(f)}</p>
            </div>
            {#if working.has(f.family)}
              <span class="state">{f.status === 'update' ? 'Updating…' : 'Downloading…'}</span>
            {:else if f.status === 'downloaded'}
              <span class="state done"><Icon name="check" size={14} />In your library</span>
            {:else if f.status === 'update'}
              <Button size="sm" variant="soft" tone="accent" icon="refresh" onclick={() => download([f.family])}>
                Update
              </Button>
            {:else}
              <Button size="sm" icon="download" onclick={() => download([f.family])}>Download</Button>
            {/if}
          </article>
        {/snippet}
      </VirtualList>
    {:else if catalog}
      <div class="empty"><p>No Google Fonts family matches.</p></div>
    {:else}
      <div class="empty"><p>Loading the Google Fonts list…</p></div>
    {/if}
  {/if}
</div>

<style>
  .google {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    height: 52px;
    padding: 0 var(--s-4) 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  .search {
    flex: 1;
    min-width: 0;
  }
  .updated {
    color: var(--muted);
    font-size: var(--text-1);
    white-space: nowrap;
  }
  .hint {
    margin: 0;
    padding: var(--s-2) var(--s-5);
    color: var(--muted);
    font-size: var(--text-1);
    border-bottom: 1px solid var(--faint);
  }

  .family {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-3) var(--s-5);
    border-bottom: 1px solid var(--faint);
    background: var(--sheet);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .specimen {
    margin: 0;
    font-size: 36px;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-synthesis: none;
  }
  .meta {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: var(--text-1);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    color: var(--muted);
    font-size: var(--text-1);
    white-space: nowrap;
  }
  .state.done {
    color: var(--cyan);
  }

  .empty {
    padding: var(--s-8) var(--s-5);
    max-width: 52ch;
    color: var(--muted);
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
  .empty h2 {
    color: var(--ink);
    font-size: var(--text-4);
    margin: 0 0 var(--s-2);
  }
  .empty p {
    margin: 0 0 var(--s-4);
    line-height: 1.55;
  }
  .row {
    display: flex;
    gap: var(--s-2);
    margin-top: var(--s-4);
  }
</style>
