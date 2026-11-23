<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type GoogleTagStatus, type Tag } from './api';
  import { ago } from './time';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import ConfirmDialog from './ui/ConfirmDialog.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import SearchField from './ui/SearchField.svelte';
  import Segmented from './ui/Segmented.svelte';

  // Every tag in one place: your own, and those imported from Google Fonts,
  // with renaming, deleting and the Google import's controls.
  interface Props {
    tags: Tag[];
    /** Families with this tag that are in the library now. */
    countOf: (t: Tag) => number;
    onNew: () => void;
    onRename: (t: Tag) => void;
    onDelete: (t: Tag) => void;
    /** Tags changed here (an import or removal); reload them. */
    onChanged: () => void;
  }
  let { tags, countOf, onNew, onRename, onDelete, onChanged }: Props = $props();

  let google = $state<GoogleTagStatus | null>(null);
  let busy = $state(false);
  let message = $state<{ tone: 'info' | 'error'; text: string } | null>(null);
  let confirmRemove = $state(false);
  let query = $state('');

  onMount(() => {
    api.googleTagsStatus().then((g) => (google = g)).catch((e) => (message = { tone: 'error', text: String(e) }));
  });

  const strengths = [
    { value: '1', label: 'Any' },
    { value: '50', label: 'Moderate' },
    { value: '80', label: 'Strong' },
  ];
  /** The preset nearest a stored strength. */
  const strengthValue = (w: number) =>
    strengths.reduce((best, s) => (Math.abs(Number(s.value) - w) < Math.abs(Number(best.value) - w) ? s : best)).value;
  let strength = $state('50');
  $effect(() => {
    if (google) strength = strengthValue(google.minWeight);
  });

  async function importTags() {
    busy = true;
    message = null;
    try {
      google = await api.importGoogleTags(Number(strength));
      onChanged();
      message = {
        tone: 'info',
        text: google.families
          ? `Tagged ${google.families.toLocaleString()} ${google.families === 1 ? 'family' : 'families'} with ${google.tags.toLocaleString()} of Google's tags.`
          : "None of your families are in Google Fonts, so no tags applied. Families you download from Google Fonts will be tagged as they arrive.",
      };
    } catch (e) {
      message = { tone: 'error', text: String(e) };
    } finally {
      busy = false;
    }
  }

  async function removeTags() {
    busy = true;
    message = null;
    try {
      google = await api.removeGoogleTags();
      onChanged();
      message = { tone: 'info', text: "Removed Google's tags. Your own tags are untouched." };
    } catch (e) {
      message = { tone: 'error', text: String(e) };
    } finally {
      busy = false;
    }
  }

  function setStrength(v: string) {
    strength = v;
    // Already importing: apply the new strength straight away.
    if (google?.enabled) importTags();
  }

  const q = $derived(query.trim().toLowerCase());
  const visible = $derived(q ? tags.filter((t) => t.name.toLowerCase().includes(q)) : tags);
  const mine = $derived(visible.filter((t) => t.source === 'user'));
  /** Google's tags by the group Google lists them under. */
  const fromGoogle = $derived.by(() => {
    const groups = new Map<string, Tag[]>();
    for (const t of visible.filter((t) => t.source === 'google')) {
      const g = t.group || 'Other';
      groups.set(g, [...(groups.get(g) ?? []), t]);
    }
    return [...groups].sort(([a], [b]) => a.localeCompare(b));
  });
  const googleCount = $derived(tags.filter((t) => t.source === 'google').length);
  const families = (n: number) => `${n.toLocaleString()} ${n === 1 ? 'family' : 'families'}`;
</script>

{#snippet row(t: Tag)}
  <li>
    <Icon name="tag" size={14} />
    <span class="tag-name">{t.name}</span>
    <span class="tag-count">{families(countOf(t))}</span>
    <IconButton icon="edit" size="sm" label="Rename the tag {t.name}" onclick={() => onRename(t)} />
    <IconButton icon="trash" size="sm" tone="danger" label="Delete the tag {t.name}" onclick={() => onDelete(t)} />
  </li>
{/snippet}

<div class="manager">
  <div class="google">
    <div class="google-head">
      <Icon name="globe" />
      <h3>Tags from Google Fonts</h3>
    </div>
    {#if google?.enabled}
      <p>
        {families(google.families)} in your library {google.families === 1 ? 'has' : 'have'}
        {google.tags.toLocaleString()} of Google's tags{#if google.importedAt}, updated {ago(google.importedAt)}{/if}.
        They're kept up to date as you download from Google Fonts and as your folders change.
      </p>
    {:else}
      <p>
        Google tags its families by structure and feeling: Humanist, Geometric, Calm, Playful and
        more. Import them onto the Google Fonts in your library, including copies you installed
        yourself.
      </p>
    {/if}
    <div class="strength">
      <div>
        <p class="label">Tag strength</p>
        <p class="help">How strongly Google must rate a tag for a family before it applies.</p>
      </div>
      <Segmented label="Tag strength" items={strengths} value={strength} onValueChange={setStrength} />
    </div>
    <div class="actions">
      {#if google?.enabled}
        <Button size="sm" variant="outline" icon="refresh" disabled={busy} onclick={importTags}>Update now</Button>
        <Button size="sm" variant="ghost" tone="danger" disabled={busy} onclick={() => (confirmRemove = true)}>
          Remove Google's tags
        </Button>
      {:else}
        <Button size="sm" variant="solid" tone="accent" icon="cloud-download" disabled={busy || !google?.keySaved} onclick={importTags}>
          Import tags
        </Button>
        {#if google && !google.keySaved}
          <span class="help">Needs a Google Fonts API key, set above.</span>
        {/if}
      {/if}
      {#if busy}<span class="help" role="status">Working…</span>{/if}
    </div>
    {#if message}
      <Callout tone={message.tone} variant="box" onDismiss={() => (message = null)}>{message.text}</Callout>
    {/if}
  </div>

  <div class="list-head">
    {#if tags.length > 10}
      <div class="filter"><SearchField bind:value={query} placeholder="Filter tags" label="Filter tags" /></div>
    {/if}
    <span class="spacer"></span>
    <Button size="sm" variant="soft" icon="tag-plus" onclick={onNew}>New tag</Button>
  </div>

  <h3 class="group">Your tags</h3>
  {#if mine.length}
    <ul class="tags">
      {#each mine as t (t.id)}{@render row(t)}{/each}
    </ul>
  {:else}
    <p class="help none">
      {q ? 'None match.' : 'None yet. Add one from a family in the inspector, or with New tag.'}
    </p>
  {/if}

  {#if googleCount}
    <h3 class="group">From Google Fonts</h3>
    {#if !fromGoogle.length}
      <p class="help none">None match.</p>
    {/if}
    {#each fromGoogle as [group, list] (group)}
      <h4>{group}</h4>
      <ul class="tags">
        {#each list as t (t.id)}{@render row(t)}{/each}
      </ul>
    {/each}
  {/if}
</div>

<ConfirmDialog
  bind:open={confirmRemove}
  title="Remove Google's tags?"
  confirmLabel="Remove tags"
  onConfirm={removeTags}
>
  Every tag imported from Google Fonts comes off your families, and new downloads won't be tagged.
  Your own tags stay. You can import them again at any time.
</ConfirmDialog>

<style>
  .manager {
    display: grid;
    gap: var(--s-3);
  }
  .google {
    display: grid;
    gap: var(--s-3);
    padding: var(--s-4);
    border: 1px solid var(--faint);
    border-radius: var(--r-3);
  }
  .google-head {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--muted);
  }
  h3 {
    margin: 0;
    font-size: var(--text-2);
    font-weight: var(--weight-strong);
    color: var(--ink);
  }
  .google p {
    margin: 0;
    line-height: 1.55;
  }
  .strength {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-4);
  }
  .label {
    margin: 0;
    font-weight: 500;
  }
  .help {
    margin: 0;
    color: var(--muted);
    font-size: var(--text-1);
  }
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2);
  }
  .list-head {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    margin-top: var(--s-2);
  }
  .filter {
    width: 220px;
  }
  .spacer {
    flex: 1;
  }
  .group {
    margin-top: var(--s-2);
  }
  h4 {
    margin: var(--s-2) 0 0;
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--muted);
  }
  .tags {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tags li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 34px;
    padding: 0 0 0 var(--s-1);
    border-bottom: 1px solid var(--faint);
    color: var(--muted);
  }
  .tag-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink);
  }
  .tag-count {
    font-size: var(--text-1);
    white-space: nowrap;
    margin-right: var(--s-2);
  }
  .none {
    padding: var(--s-1) 0;
  }
</style>
