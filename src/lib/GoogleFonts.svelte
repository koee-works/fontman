<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import {
    api,
    type CatalogView,
    type DownloadProgress,
    type FamilySummary,
    type GoogleFamilySummary,
  } from './api';
  import { categoryLabel, googleFacets, googleSorts, scriptLabel, splitTag } from './facets';
  import FilterBar from './filters/FilterBar.svelte';
  import { googleFullId, googlePreviewId, setFaceSizes, specimen } from './fonts';
  import {
    applyFacets,
    closeMatches,
    isEmptyQuery,
    matchQuery,
    parseQuery,
    relevance,
    sortItems,
  } from './query';
  import { QueryState } from './query-state.svelte';
  import { ago } from './time';
  import VirtualList from './VirtualList.svelte';
  import ActiveToggle from './ui/ActiveToggle.svelte';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import NumberField from './ui/NumberField.svelte';
  import SearchField from './ui/SearchField.svelte';
  import Segmented from './ui/Segmented.svelte';
  import Slider from './ui/Slider.svelte';
  import Waterfall from './Waterfall.svelte';

  type Layout = 'list' | 'grid';

  interface Props {
    keySaved: boolean;
    onOpenSettings: () => void;
    /** Called after downloads so the library list picks up the new fonts. */
    onDownloaded: () => Promise<void> | void;
    /** The library, to show whether downloaded families are active. */
    library: FamilySummary[];
    busy: boolean;
    onToggleFamily: (family: FamilySummary, active: boolean) => void;
    /** Shared with the library: the same text, size and layout. */
    sample: string;
    size: number;
    layout: Layout;
    onLayout: (layout: Layout) => void;
  }
  let {
    keySaved,
    onOpenSettings,
    onDownloaded,
    library,
    busy,
    onToggleFamily,
    sample = $bindable(),
    size = $bindable(),
    layout,
    onLayout,
  }: Props = $props();

  let catalog = $state.raw<CatalogView | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let query = $state('');
  let working = $state.raw(new Map<string, 'download' | 'activate' | 'update'>());
  let progress = $state<DownloadProgress | null>(null);
  let notice = $state<{ tone: 'info' | 'error'; text: string } | null>(null);
  /** Families opened to show a waterfall and the alphabet. */
  let opened = $state.raw(new Set<string>());

  // -- The library, for activation -------------------------------------------

  const byName = $derived(new Map(library.map((f) => [f.name.toLowerCase(), f])));
  const inLibrary = (g: GoogleFamilySummary) =>
    g.status === 'available' ? undefined : byName.get((g.library ?? g.family).toLowerCase());

  // -- Search, filters, order -------------------------------------------------

  const parsed = $derived(parseQuery(query));
  const searching = $derived(parsed.include.length > 0);
  const gq = new QueryState('google-query', { key: 'popular', dir: 'desc' });
  const facets = $derived(googleFacets(inLibrary));
  const sorts = googleSorts((f) => relevance(parsed, f.family));
  const sort = $derived(gq.effective(searching));
  $effect(() => {
    if (!searching) gq.endSearch();
  });

  /** Search matches the name, category, scripts and Google's tags. */
  const fields = (f: GoogleFamilySummary) => [
    f.family,
    categoryLabel(f.category),
    ...f.subsets.map(scriptLabel),
    ...f.tags.map((t) => splitTag(t).label),
  ];
  const searched = $derived.by(() => {
    if (!catalog) return [];
    if (isEmptyQuery(parsed)) return catalog.families;
    return catalog.families.filter((f) => matchQuery(parsed, fields(f)));
  });
  const close = $derived(
    catalog && searching && searched.length === 0
      ? closeMatches(catalog.families, parsed, (f) => f.family)
      : [],
  );
  const shown = $derived.by(() => {
    if (close.length) return close;
    return sortItems(applyFacets(searched, facets, gq.filters), sorts, sort, (f) => f.family);
  });
  const updates = $derived(catalog?.families.filter((f) => f.status === 'update') ?? []);

  /** Typed text needs the full file; the family's name needs only a few glyphs. */
  const typed = $derived(sample.trim().length > 0);
  const previewId = (f: GoogleFamilySummary) => (typed ? googleFullId(f.family) : googlePreviewId(f.family));
  const previewText = (f: GoogleFamilySummary) => (typed ? sample : f.family);

  // -- Loading and downloading ------------------------------------------------

  async function load(refresh = false) {
    if (!keySaved) return;
    loading = true;
    error = null;
    try {
      const view = await api.googleCatalog(refresh);
      // Name-only previews are a few KB, full regular styles a few hundred.
      setFaceSizes(view.families.map((f) => ({ id: googlePreviewId(f.family), bytes: 8192 })));
      setFaceSizes(view.families.map((f) => ({ id: googleFullId(f.family), bytes: 320 * 1024 })));
      catalog = view;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function setWorking(families: string[], what: 'download' | 'activate' | 'update' | null) {
    const next = new Map(working);
    for (const f of families) {
      if (what) next.set(f, what);
      else next.delete(f);
    }
    working = next;
  }

  async function download(families: string[], activate = false) {
    if (!families.length) return;
    const isUpdate = families.every(
      (name) => catalog?.families.find((f) => f.family === name)?.status === 'update',
    );
    setWorking(families, isUpdate ? 'update' : activate ? 'activate' : 'download');
    notice = null;
    try {
      const results = await api.googleDownload(families, activate);
      const ok = results.filter((r) => r.ok).map((r) => r.family);
      const failed = results.filter((r) => !r.ok);
      await load();
      await onDownloaded();
      const what = ok.length === 1 ? ok[0] : `${ok.length} families`;
      if (failed.length) {
        notice = {
          tone: 'error',
          text: `${failed[0]!.family}: ${failed[0]!.error}${failed.length > 1 ? ` (and ${failed.length - 1} more)` : ''}`,
        };
      } else if (ok.length) {
        notice = {
          tone: 'info',
          text: isUpdate
            ? `Updated ${what}.`
            : activate
              ? `Downloaded and activated ${what}. Apps opened from now on can use ${ok.length === 1 ? 'it' : 'them'}.`
              : `Downloaded ${what} to your library. Switch ${ok.length === 1 ? 'it' : 'them'} on here or in My library.`,
        };
      }
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      setWorking(families, null);
      progress = null;
    }
  }

  function toggleOpen(family: string) {
    const next = new Set(opened);
    if (next.has(family)) next.delete(family);
    else next.add(family);
    opened = next;
  }

  function meta(f: GoogleFamilySummary): string {
    const parts = [categoryLabel(f.category), f.styles === 1 ? '1 style' : `${f.styles} styles`];
    if (f.variable) parts.push(f.axes.length > 1 ? `variable, ${f.axes.length} axes` : 'variable');
    parts.push(f.subsets.length === 1 ? scriptLabel(f.subsets[0]!) : `${f.subsets.length} scripts`);
    return parts.join(', ');
  }

  const pageUrl = (f: GoogleFamilySummary) =>
    `https://fonts.google.com/specimen/${encodeURIComponent(f.family).replace(/%20/g, '+')}`;

  // -- Grid geometry, as in the library ---------------------------------------

  const CARD_MIN = 260;
  const GRID_GAP = 16;
  const GRID_PAD = 20;
  let listWidth = $state(0);
  const columns = $derived(
    Math.max(1, Math.floor((listWidth - GRID_PAD * 2 + GRID_GAP) / (CARD_MIN + GRID_GAP))),
  );
  const gridRows = $derived.by(() => {
    const rows: { key: string; families: GoogleFamilySummary[]; last: boolean }[] = [];
    for (let i = 0; i < shown.length; i += columns) {
      const families = shown.slice(i, i + columns);
      rows.push({ key: families[0]!.family, families, last: i + columns >= shown.length });
    }
    return rows;
  });

  const resetKey = $derived(`${query}|${JSON.stringify(gq.filters)}|${sort.key}${sort.dir}|${layout}`);

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

{#snippet actions(f: GoogleFamilySummary)}
  {@const lib = inLibrary(f)}
  {@const job = working.get(f.family)}
  <div class="actions">
    {#if job}
      <span class="state">
        {job === 'update' ? 'Updating…' : job === 'activate' ? 'Downloading and activating…' : 'Downloading…'}
      </span>
    {:else if f.status === 'available'}
      <Button size="sm" variant="ghost" icon="download" onclick={() => download([f.family])}>Download</Button>
      <Button size="sm" variant="soft" tone="accent" icon="active" onclick={() => download([f.family], true)}>
        Activate
      </Button>
    {:else}
      {#if f.status === 'update'}
        <Button size="sm" variant="soft" tone="accent" icon="refresh" onclick={() => download([f.family])}>
          Update
        </Button>
      {:else}
        <span class="state done">In your library</span>
      {/if}
      {#if lib}
        <ActiveToggle
          state={lib.installed ? 'installed' : lib.activation}
          name={lib.name}
          on={lib.faces.filter((x) => x.active).length}
          total={lib.faces.length}
          disabled={busy}
          onToggle={(active) => onToggleFamily(lib, active)}
        />
      {/if}
    {/if}
  </div>
{/snippet}

{#snippet details(f: GoogleFamilySummary)}
  <div class="details">
    <Waterfall faceId={googleFullId(f.family)} text={typed ? sample : 'The quick brown fox jumps over the lazy dog'} variation="normal" />
    <p class="alphabet" use:specimen={googleFullId(f.family)}>
      ABCDEFGHIJKLMNOPQRSTUVWXYZ<br />abcdefghijklmnopqrstuvwxyz<br />0123456789 &amp;@?!%(.,;:)
    </p>
    <dl class="facts">
      <div><dt>Category</dt><dd>{categoryLabel(f.category)}</dd></div>
      <div><dt>Styles</dt><dd>{f.styles}{f.italic ? ', with italics' : ''}</dd></div>
      {#if f.axes.length}<div><dt>Axes</dt><dd>{f.axes.join(', ')}</dd></div>{/if}
      <div><dt>Scripts</dt><dd>{f.subsets.map(scriptLabel).join(', ')}</dd></div>
      {#if f.tags.length}<div><dt>Google's tags</dt><dd>{f.tags.map((t) => splitTag(t).label).join(', ')}</dd></div>{/if}
      <div><dt>Version</dt><dd>{f.version}, updated {f.lastModified}</dd></div>
    </dl>
    <Button size="sm" variant="ghost" icon="external" onclick={() => openUrl(pageUrl(f))}>
      Open on Google Fonts
    </Button>
  </div>
{/snippet}

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
        <SearchField
          bind:value={query}
          size="lg"
          placeholder={'Search Google Fonts: names, scripts, tags. "Exact phrase", -exclude'}
          label="Search Google Fonts"
        />
      </div>
      <span class="updated" aria-live="polite">
        {#if loading}Loading…{:else if catalog}Updated {ago(catalog.fetchedAt)}{/if}
      </span>
      <IconButton icon="refresh" label="Refresh the font list" disabled={loading} onclick={() => load(true)} />
    </div>

    <div class="tester">
      <label class="sample">
        <span class="visually-hidden">Sample text</span>
        <input
          type="text"
          placeholder="Type to preview. Empty shows each family's name."
          bind:value={sample}
          spellcheck="false"
        />
      </label>
      <Segmented
        label="Layout"
        iconOnly
        items={[
          { value: 'list', label: 'List', icon: 'list' },
          { value: 'grid', label: 'Grid', icon: 'grid' },
        ]}
        value={layout}
        onValueChange={onLayout}
      />
      <div class="size">
        <Slider bind:value={size} min={12} max={120} label="Preview size" />
        <NumberField bind:value={size} min={12} max={120} unit="px" label="Preview size" />
      </div>
    </div>

    {#if catalog}
      <FilterBar
        items={searched}
        {facets}
        query={gq}
        {sorts}
        {sort}
        onSort={(x) => gq.pick(x, searching)}
        {searching}
        shown={shown.length}
        noun={['family', 'families']}
      />
    {/if}

    {#if catalog?.stale}
      <Callout tone="warning">{catalog.stale}</Callout>
    {/if}
    {#if close.length}
      <Callout>
        No family matches “{query.trim()}”. Showing names close to it.
        {#snippet action()}
          <Button size="sm" variant="ghost" onclick={() => (query = '')}>Clear search</Button>
        {/snippet}
      </Callout>
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
      <div class="results" bind:clientWidth={listWidth}>
        {#if layout === 'list'}
          <VirtualList
            items={shown}
            key={(f) => f.family}
            estimate={(f) => (opened.has(f.family) ? 640 : 40 + size * 1.25)}
            resetScroll={resetKey}
            resetMeasure={`${size}|${typed}`}
            label="Google Fonts families"
          >
            {#snippet row(f)}
              <article class="family" class:open={opened.has(f.family)}>
                <div class="head">
                  <button
                    type="button"
                    class="disclose"
                    aria-expanded={opened.has(f.family)}
                    aria-label="{opened.has(f.family) ? 'Hide' : 'Show'} a closer look at {f.family}"
                    onclick={() => toggleOpen(f.family)}
                  >
                    <Icon name={opened.has(f.family) ? 'chevron-down' : 'chevron-right'} size={14} />
                  </button>
                  <div class="text">
                    <p class="specimen" style:font-size="{size}px" use:specimen={previewId(f)}>{previewText(f)}</p>
                    <p class="meta"><span class="name">{f.family}</span>{meta(f)}</p>
                  </div>
                  {@render actions(f)}
                </div>
                {#if opened.has(f.family)}
                  {@render details(f)}
                {/if}
              </article>
            {/snippet}
          </VirtualList>
        {:else}
          <VirtualList
            items={gridRows}
            key={(r) => `${columns}:${r.key}`}
            estimate={() => GRID_GAP + 2 + 28 + size * 2.4 + 1 + 24 + 44}
            resetScroll={resetKey}
            resetMeasure={`${size}|${columns}|${typed}`}
            label="Google Fonts families"
          >
            {#snippet row(r)}
              <div class="grid-row" class:last={r.last} style:grid-template-columns="repeat({columns}, minmax(0, 1fr))">
                {#each r.families as f (f.family)}
                  <article class="card">
                    <p class="card-specimen" style:font-size="{size}px" use:specimen={previewId(f)}>{previewText(f)}</p>
                    <div class="card-foot">
                      <div class="card-text">
                        <p class="card-name">{f.family}</p>
                        <p class="meta">{meta(f)}</p>
                      </div>
                    </div>
                    {@render actions(f)}
                  </article>
                {/each}
              </div>
            {/snippet}
          </VirtualList>
        {/if}
      </div>
    {:else if catalog && searched.length}
      <div class="empty">
        <p>
          {searched.length.toLocaleString()} {searched.length === 1 ? 'family matches' : 'families match'},
          but the filters hide {searched.length === 1 ? 'it' : 'all of them'}.
        </p>
        <Button variant="ghost" onclick={() => gq.clear()}>Clear filters</Button>
      </div>
    {:else if catalog}
      <div class="empty">
        <p>No Google Fonts family matches “{query.trim()}”.</p>
        <Button variant="ghost" onclick={() => (query = '')}>Clear search</Button>
      </div>
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

  /* Same as the library's sample row. */
  .tester {
    display: flex;
    align-items: center;
    gap: var(--s-5);
    height: 52px;
    padding: 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  .sample {
    flex: 1;
    min-width: 0;
  }
  .sample input {
    width: 100%;
    border: 0;
    background: none;
    font-size: var(--text-3);
    padding: var(--s-1) 0;
    outline-offset: 4px;
  }
  .sample input::placeholder {
    color: var(--muted);
  }
  .size {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 200px;
    flex: none;
    color: var(--muted);
  }

  .results {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .family {
    border-bottom: 1px solid var(--faint);
    background: var(--sheet);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4) var(--s-3) var(--s-3);
  }
  .disclose {
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: var(--r-1);
    color: var(--muted);
  }
  .disclose:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .specimen {
    margin: 0;
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta .name {
    color: var(--ink);
    font-weight: 500;
    margin-right: var(--s-1);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    flex: none;
  }
  .state {
    color: var(--muted);
    font-size: var(--text-1);
    white-space: nowrap;
  }
  .state.done {
    color: var(--cyan);
  }

  .details {
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    padding: 0 var(--s-5) var(--s-5) calc(var(--s-3) + 22px + var(--s-3));
  }
  .alphabet {
    margin: 0;
    font-size: 28px;
    line-height: 1.3;
    font-synthesis: none;
    word-break: break-all;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--s-2) var(--s-5);
    margin: 0;
    font-size: var(--text-1);
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 2px 0 0;
  }
  .details > :global(.fm-btn) {
    align-self: flex-start;
  }

  .grid-row {
    display: grid;
    gap: var(--s-4);
    padding: var(--s-4) var(--s-5) 0;
  }
  .grid-row.last {
    padding-bottom: var(--s-4);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    min-width: 0;
    padding: var(--s-4);
    border: 1px solid var(--faint);
    border-radius: var(--r-3);
    background: var(--sheet);
  }
  .card-specimen {
    margin: 0;
    line-height: 1.2;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow-wrap: anywhere;
    font-synthesis: none;
  }
  .card-text {
    min-width: 0;
  }
  .card-name {
    margin: 0;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .card .actions {
    margin-top: auto;
    padding-top: var(--s-2);
    border-top: 1px solid var(--faint);
    justify-content: flex-end;
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
