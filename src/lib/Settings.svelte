<script lang="ts" module>
  export type SettingsPage = 'appearance' | 'library' | 'google' | 'tags' | 'about';
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener';
  import { updates, RELEASES_URL } from './updates.svelte';
  import { api, type GoogleSettings, type Tag, type WatchStatus } from './api';
  import TagManager from './TagManager.svelte';
  import { theme, type Appearance, type ThemePreference } from './theme.svelte';
  import { motion, type Motion } from './motion.svelte';
  import { fuzzy } from './palette';
  import SearchField from './ui/SearchField.svelte';
  import Select from './ui/Select.svelte';
  import type { IconName } from './ui/Icon.svelte';
  import { ago } from './time';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import ConfirmDialog from './ui/ConfirmDialog.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import Segmented from './ui/Segmented.svelte';

  interface Props {
    /** Called after the key is saved or removed, so other views can update. */
    onKeyChanged?: (settings: GoogleSettings) => void;
    tags: Tag[];
    tagCount: (t: Tag) => number;
    onNewTag: () => void;
    onRenameTag: (t: Tag) => void;
    onDeleteTag: (t: Tag) => void;
    onTagsChanged: () => void;
    /** The page shown; the app opens About to show an update. */
    page?: Page;
  }
  let {
    onKeyChanged,
    tags,
    tagCount,
    onNewTag,
    onRenameTag,
    onDeleteTag,
    onTagsChanged,
    page = $bindable('appearance'),
  }: Props = $props();

  const links = {
    console: 'https://console.cloud.google.com/',
    enable: 'https://console.cloud.google.com/apis/library/webfonts.googleapis.com',
    credentials: 'https://console.cloud.google.com/apis/credentials',
    docs: 'https://developers.google.com/fonts/docs/developer_api',
  };

  let settings = $state<GoogleSettings | null>(null);
  let watch = $state<WatchStatus | null>(null);
  let watchError = $state<string | null>(null);

  async function setWatching(on: boolean) {
    watchError = null;
    try {
      watch = await api.setWatching(on);
    } catch (e) {
      watchError = String(e);
    }
  }
  let keyInput = $state('');
  let reveal = $state(false);
  let editing = $state(false);
  let saving = $state(false);
  let confirmRemove = $state(false);
  let message = $state<{ tone: 'info' | 'warning' | 'error'; text: string } | null>(null);
  let input = $state<HTMLInputElement>();

  const where = $derived(
    settings?.storage === 'file'
      ? "in a private file in the app's data folder, because no system keychain was available"
      : 'in your system keychain',
  );

  onMount(async () => {
    api.libraryStatus().then((st) => (watch = st.watch)).catch(() => {});
    try {
      settings = await api.googleSettings();
    } catch (e) {
      message = { tone: 'error', text: String(e) };
    }
  });

  function open(url: string) {
    return (e: MouseEvent) => {
      e.preventDefault();
      openUrl(url).catch(
        (err) => (message = { tone: 'error', text: `Couldn't open the browser: ${err}` }),
      );
    };
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    saving = true;
    message = null;
    try {
      const r = await api.saveGoogleKey(keyInput);
      settings = r.settings;
      keyInput = '';
      reveal = false;
      editing = false;
      message = r.warning
        ? { tone: 'warning', text: r.warning }
        : { tone: 'info', text: 'Key saved. Google Fonts is ready to use.' };
      onKeyChanged?.(r.settings);
    } catch (err) {
      message = { tone: 'error', text: String(err) };
      input?.focus();
    } finally {
      saving = false;
    }
  }

  async function remove() {
    message = null;
    try {
      settings = await api.removeGoogleKey();
      editing = false;
      message = { tone: 'info', text: 'Key removed.' };
      onKeyChanged?.(settings);
    } catch (err) {
      message = { tone: 'error', text: String(err) };
    }
  }

  function startReplace() {
    editing = true;
    message = null;
    queueMicrotask(() => input?.focus());
  }

  // -- Pages and search, after Zed's settings window -------------------------

  type Page = SettingsPage;
  const pages: { id: Page; label: string; icon: IconName }[] = [
    { id: 'appearance', label: 'Appearance', icon: 'palette' },
    { id: 'library', label: 'Library', icon: 'library' },
    { id: 'google', label: 'Google Fonts', icon: 'globe' },
    { id: 'tags', label: 'Tags', icon: 'tags' },
    { id: 'about', label: 'About', icon: 'info' },
  ];
  let query = $state('');
  let body = $state<HTMLDivElement>();
  let flash = $state<string | null>(null);

  /** Every setting, for search: where it lives and what it's called. */
  const index: { page: Page; id: string; title: string; words: string }[] = [
    { page: 'appearance', id: 'set-mode', title: 'Mode', words: 'theme light dark system appearance' },
    { page: 'appearance', id: 'set-light-theme', title: 'Light theme', words: 'colours colors zed one ayu gruvbox' },
    { page: 'appearance', id: 'set-dark-theme', title: 'Dark theme', words: 'colours colors zed one ayu gruvbox' },
    { page: 'appearance', id: 'set-motion', title: 'Motion', words: 'animation animations speed snappy graceful none reduce reduced transitions' },
    { page: 'appearance', id: 'set-import-theme', title: 'Import a Zed theme', words: 'theme file json' },
    { page: 'library', id: 'set-watch', title: 'Watch folders for changes', words: 'rescan automatic notify' },
    { page: 'google', id: 'set-api-key', title: 'Google Fonts API key', words: 'key google cloud' },
    { page: 'google', id: 'set-downloads', title: 'Download folder', words: 'google fonts downloads' },
    { page: 'tags', id: 'set-google-tags', title: 'Tags from Google Fonts', words: 'import strength' },
    { page: 'tags', id: 'set-tags', title: 'Your tags', words: 'rename delete new tag' },
    { page: 'about', id: 'set-version', title: 'Version and updates', words: 'about update upgrade new version check install restart' },
    { page: 'about', id: 'set-auto-update', title: 'Check for updates at startup', words: 'automatic update' },
    { page: 'about', id: 'set-backups', title: 'Your data and backups', words: 'backup restore settings library folder data' },
    { page: 'about', id: 'set-credits', title: 'Theme credits and licences', words: 'mit licence license' },
    { page: 'about', id: 'set-icons', title: 'Icons', words: 'lucide feather isc licence license credits' },
  ];
  const results = $derived.by(() => {
    const q = query.trim();
    if (!q) return [];
    return index
      .map((e) => ({ e, m: fuzzy(q, e.title) ?? fuzzy(q, `${e.title} ${e.words}`) }))
      .filter((r) => r.m)
      .sort((a, b) => b.m!.score - a.m!.score)
      .map((r) => r.e);
  });
  const pageLabel = (p: Page) => pages.find((x) => x.id === p)!.label;

  /** Show a setting: open its page, scroll to it and highlight it briefly. */
  function jump(p: Page, id: string) {
    page = p;
    query = '';
    queueMicrotask(() => {
      const el = body?.querySelector<HTMLElement>(`#${id}`);
      el?.scrollIntoView({ block: 'center' });
      flash = id;
      setTimeout(() => (flash = null), 1200);
    });
  }
  function show(p: Page) {
    page = p;
    body?.scrollTo({ top: 0 });
  }

  let themeError = $state<string | null>(null);
  async function importTheme() {
    themeError = null;
    try {
      const u = await theme.importFile();
      if (u) {
        // Offer the new theme straight away: pick its first of each kind.
        const first = (a: Appearance) => u.family.themes.find((t) => t.appearance === a);
        const light = first('light');
        const dark = first('dark');
        if (light) theme.choose('light', `user:${u.file}:${light.name}`);
        if (dark) theme.choose('dark', `user:${u.file}:${dark.name}`);
      }
    } catch (e) {
      themeError = String(e);
    }
  }

  const themes: { value: ThemePreference; label: string; icon: 'monitor' | 'sun' | 'moon' }[] = [
    { value: 'system', label: 'System', icon: 'monitor' },
    { value: 'light', label: 'Light', icon: 'sun' },
    { value: 'dark', label: 'Dark', icon: 'moon' },
  ];
</script>

<div class="settings">
  <nav class="pages" aria-label="Settings pages">
    <SearchField bind:value={query} placeholder="Search settings" label="Search settings" />
    {#if query.trim()}
      <ul class="results" aria-label="Matching settings">
        {#each results as r (r.id)}
          <li>
            <button onclick={() => jump(r.page, r.id)}>
              <span>{r.title}</span>
              <span class="where">{pageLabel(r.page)}</span>
            </button>
          </li>
        {:else}
          <li class="none">No setting matches.</li>
        {/each}
      </ul>
    {:else}
      <ul>
        {#each pages as p (p.id)}
          <li>
            <button class:current={page === p.id} aria-current={page === p.id ? 'page' : undefined} onclick={() => show(p.id)}>
              <Icon name={p.icon} />
              {p.label}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </nav>

  <div class="body" bind:this={body}>
    <h2 class="page-title">{pageLabel(page)}</h2>

    {#if page === 'appearance'}
      <div class="setting" id="set-mode" class:flash={flash === 'set-mode'}>
        <div class="text">
          <p class="label">Mode</p>
          <p class="help">System follows your computer's light or dark setting.</p>
        </div>
        <Segmented label="Mode" items={themes} value={theme.preference} onValueChange={(v) => theme.set(v)} />
      </div>
      {#each [{ a: 'light', id: 'set-light-theme', label: 'Light theme' }, { a: 'dark', id: 'set-dark-theme', label: 'Dark theme' }] as row (row.a)}
        {@const a = row.a as Appearance}
        <div class="setting" id={row.id} class:flash={flash === row.id}>
          <div class="text">
            <p class="label">{row.label}</p>
            <p class="help">Used when the app is {a}{theme.appearance === a ? ', as it is now' : ''}.</p>
          </div>
          <Select
            label={row.label}
            items={theme.options(a).map((o) => ({ value: o.id, label: o.label }))}
            value={a === 'dark' ? theme.dark : theme.light}
            onValueChange={(v) => theme.choose(a, v)}
          />
        </div>
      {/each}
      {#if theme.adjusted.length}
        <p class="help note">
          To keep text readable, some colours of this theme were adjusted: {theme.adjusted.join(', ')}.
        </p>
      {/if}
      <div class="setting" id="set-motion" class:flash={flash === 'set-motion'}>
        <div class="text">
          <p class="label">Motion</p>
          <p class="help">
            How panes open and close, fonts fade in as they load, the command palette appears and
            tabs change.
            {#if motion.choice === null && motion.systemReduce}Off for now, because your computer asks for reduced motion.{/if}
          </p>
        </div>
        <Segmented
          label="Motion"
          items={[
            { value: 'none', label: 'None' },
            { value: 'snappy', label: 'Snappy' },
            { value: 'graceful', label: 'Graceful' },
          ]}
          value={motion.current}
          onValueChange={(v) => motion.set(v as Motion)}
        />
      </div>
      <div class="setting" id="set-import-theme" class:flash={flash === 'set-import-theme'}>
        <div class="text">
          <p class="label">Zed themes</p>
          <p class="help">
            Use any theme made for the Zed editor: get its <code>.json</code> file, then import it. One,
            Ayu and Gruvbox come with the app.
          </p>
        </div>
        <Button size="sm" icon="import-file" onclick={importTheme}>Import theme…</Button>
      </div>
      {#if themeError}<Callout tone="error" variant="box">{themeError}</Callout>{/if}
      {#if theme.imported.length}
        <ul class="imported" aria-label="Imported themes">
          {#each theme.imported as u (u.file)}
            <li>
              <span class="fam">{u.family.name}</span>
              <span class="help">
                {u.family.themes.map((t) => t.name).join(', ')}{u.family.author ? `, by ${u.family.author}` : ''}
              </span>
              <IconButton icon="trash" size="sm" tone="danger" label="Remove {u.family.name}" onclick={() => theme.remove(u.file)} />
            </li>
          {/each}
        </ul>
      {/if}
    {:else if page === 'library'}
      <div class="setting" id="set-watch" class:flash={flash === 'set-watch'}>
        <div class="text">
          <p class="label">Watch folders for changes</p>
          <p class="help">
            Fonts added to, changed in or removed from your folders show up within a few seconds,
            without a manual check.
          </p>
        </div>
        {#if watch}
          <Segmented
            label="Watch folders for changes"
            items={[
              { value: 'on', label: 'On' },
              { value: 'off', label: 'Off' },
            ]}
            value={watch.enabled ? 'on' : 'off'}
            onValueChange={(v) => setWatching(v === 'on')}
          />
        {/if}
      </div>
      {#if watch?.enabled && !watch.error}
        <p class="help note">
          Watching {watch.folders.toLocaleString()} {watch.folders === 1 ? 'folder' : 'folders'},
          including the folders inside them. Network drives may not report changes; use Check for
          changes in the status bar for those.
        </p>
      {/if}
      {#if watch?.error}
        <Callout tone="warning" variant="box">{watch.error}</Callout>
      {/if}
      {#if watchError}
        <Callout tone="error" variant="box">{watchError}</Callout>
      {/if}
    {:else if page === 'google'}
      <p>
        Browse and download the Google Fonts library from inside the app. Google asks for a
        free API key for this, which you create once in Google Cloud. It takes about five
        minutes.
      </p>

      <ol class="steps">
        <li>
          Open the <a href={links.console} onclick={open(links.console)}>Google Cloud console</a>
          and sign in. Create a project if you don't have one; any name works.
        </li>
        <li>
          <a href={links.enable} onclick={open(links.enable)}>Enable the Web Fonts Developer API</a>
          for that project.
        </li>
        <li>
          Go to <a href={links.credentials} onclick={open(links.credentials)}>Credentials</a>,
          choose Create credentials, then API key.
        </li>
        <li>
          Recommended: edit the new key and, under API restrictions, limit it to the Web Fonts
          Developer API. Then a leaked key can't be used for anything else.
        </li>
        <li>Copy the key and paste it below.</li>
      </ol>

      <div class="card" id="set-api-key" class:flash={flash === 'set-api-key'}>
        {#if settings?.keySaved && !editing}
          <div class="saved">
            <span class="key-icon"><Icon name="lock" /></span>
            <p>
              Key ending in <strong>{settings.keyEnding}</strong> is saved {where}. It is only
              ever sent to Google.
            </p>
          </div>
          <div class="actions">
            <Button onclick={startReplace}>Replace key</Button>
            <Button tone="danger" icon="trash" onclick={() => (confirmRemove = true)}>
              Remove key
            </Button>
          </div>
        {:else}
          <form onsubmit={save}>
            <label for="api-key" class="label">API key</label>
            <div class="field">
              <input
                id="api-key"
                bind:this={input}
                bind:value={keyInput}
                type={reveal ? 'text' : 'password'}
                autocomplete="off"
                spellcheck="false"
                placeholder="Paste your key"
                aria-describedby="key-help"
                disabled={saving}
              />
              <IconButton
                icon={reveal ? 'eye-off' : 'eye'}
                label={reveal ? 'Hide key' : 'Show key'}
                pressed={reveal}
                onclick={() => (reveal = !reveal)}
              />
            </div>
            <p id="key-help" class="help">
              Checked with Google before it's saved, then kept in your system keychain where
              available.
            </p>
            <div class="actions">
              <Button
                type="submit"
                variant="solid"
                tone="accent"
                disabled={saving || !keyInput.trim()}
              >
                {saving ? 'Checking with Google…' : 'Save key'}
              </Button>
              {#if editing}
                <Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button>
              {/if}
            </div>
          </form>
        {/if}

        {#if message}
          <div class="message">
            <Callout tone={message.tone} variant="box">{message.text}</Callout>
          </div>
        {/if}
      </div>

      {#if settings}
        <dl class="facts" id="set-downloads" class:flash={flash === 'set-downloads'}>
          <dt>Downloads go to</dt>
          <dd class="path">{settings.downloadFolder}</dd>
          {#if settings.catalogFetchedAt && settings.catalogFamilies}
            <dt>Font list</dt>
            <dd>
              {settings.catalogFamilies.toLocaleString()} families, updated {ago(settings.catalogFetchedAt)}
            </dd>
          {/if}
        </dl>
      {/if}

      <p class="more">
        More about the API in
        <a href={links.docs} onclick={open(links.docs)}>Google's documentation<Icon name="external" size={12} /></a>.
      </p>
    {:else if page === 'tags'}
      <p class="help intro">
        Tags describe families: a style, a use, a client. They show on every row and card, and
        search finds them.
      </p>
      <div id="set-google-tags" class:flash={flash === 'set-google-tags' || flash === 'set-tags'}>
        <TagManager
          {tags}
          countOf={tagCount}
          onNew={onNewTag}
          onRename={onRenameTag}
          onDelete={onDeleteTag}
          onChanged={() => {
            onTagsChanged();
            // An import may have fetched a fresh font list.
            api.googleSettings().then((g) => (settings = g)).catch(() => {});
          }}
        />
      </div>
    {:else}
      {@const u = updates}
      <div class="setting" id="set-version" class:flash={flash === 'set-version'}>
        <div class="text">
          <p class="label">Font Manager {u.info?.version ?? ''}</p>
          <p class="help" aria-live="polite">
            {#if u.info && !u.info.canSelfUpdate}
              This copy was installed as a Linux package, which can't replace itself. New versions
              are on GitHub; installing one keeps your library and settings.
            {:else if u.status === 'checking'}
              Checking for updates…
            {:else if u.status === 'current'}
              You have the latest version.
            {:else if u.status === 'available' && u.available}
              Version {u.available.version} is available.
            {:else if u.status === 'downloading'}
              Downloading {u.available?.version}{u.total ? `: ${Math.round((u.received / u.total) * 100)}%` : '…'}
            {:else if u.status === 'installing'}
              Installing. Font Manager will restart.
            {:else if u.status === 'error'}
              <span class="error-text">{u.error}</span>
            {:else}
              Updates come from this project's releases on GitHub.
            {/if}
          </p>
        </div>
        {#if u.info && !u.info.canSelfUpdate}
          <Button size="sm" icon="external" onclick={() => openUrl(RELEASES_URL)}>Open releases</Button>
        {:else if u.status === 'available'}
          <Button size="sm" variant="solid" tone="accent" icon="download" onclick={() => u.install()}>
            Update and restart
          </Button>
        {:else}
          <Button
            size="sm"
            icon="refresh"
            disabled={u.status === 'checking' || u.status === 'downloading' || u.status === 'installing'}
            onclick={() => u.check()}
          >
            Check for updates
          </Button>
        {/if}
      </div>
      {#if u.status === 'available' && u.available?.notes}
        <div class="notes">
          <p class="label">What's new in {u.available.version}</p>
          <p class="help">{u.available.notes}</p>
        </div>
      {/if}
      {#if u.info?.canSelfUpdate}
        <div class="setting" id="set-auto-update" class:flash={flash === 'set-auto-update'}>
          <div class="text">
            <p class="label">Check for updates at startup</p>
            <p class="help">Nothing installs without you choosing Update and restart.</p>
          </div>
          <Segmented
            label="Check for updates at startup"
            items={[
              { value: 'on', label: 'On' },
              { value: 'off', label: 'Off' },
            ]}
            value={u.auto ? 'on' : 'off'}
            onValueChange={(v) => u.setAuto(v === 'on')}
          />
        </div>
      {/if}
      <div class="setting" id="set-backups" class:flash={flash === 'set-backups'}>
        <div class="text">
          <p class="label">Your data and backups</p>
          <p class="help">
            Your folders, collections, tags, activated fonts and settings are kept in the app's data
            folder, which updates and reinstalls leave alone. The first time a new version starts,
            it copies them into a backup first; the last five are kept.
            {#if u.info?.updatedFrom}
              This version was updated from {u.info.updatedFrom === 'earlier' ? 'an earlier version' : u.info.updatedFrom},
              and a backup was made.
            {/if}
          </p>
        </div>
        {#if u.info}
          <Button size="sm" icon="reveal" onclick={() => u.info && revealItemInDir(u.info.backupsDir).catch(() => revealItemInDir(u.info!.dataDir).catch(() => {}))}>
            Show backups
          </Button>
        {/if}
      </div>
      <div class="setting" id="set-credits" class:flash={flash === 'set-credits'}>
        <div class="text">
          <p class="label">Theme credits</p>
          <p class="help">
            The bundled themes are the UI colours of the themes that ship with the Zed editor, each
            under the MIT licence: One (GitHub, from Atom), Ayu (Ike Ku) and Gruvbox (Pavel
            Pertsev). The full notices are in the app's source, in
            <code>src/lib/themes/LICENSES.md</code>.
          </p>
        </div>
      </div>
      <div class="setting" id="set-icons" class:flash={flash === 'set-icons'}>
        <div class="text">
          <p class="label">Icons</p>
          <p class="help">
            Icons are from Lucide (lucide.dev), under the ISC licence, with some derived from
            Feather under the MIT licence. The full notice is in the app's source, in
            <code>licenses/LUCIDE.txt</code>.
          </p>
        </div>
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:open={confirmRemove}
  title="Remove the API key?"
  confirmLabel="Remove key"
  onConfirm={remove}
>
  Google Fonts will stop working until you add a key again. Fonts you've already downloaded
  stay in your library.
</ConfirmDialog>

<style>
  /* Zed's settings window: pages on the left, settings on the right, each a
     row with its name and description on the left and its control on the
     right, separated by hairlines. */
  .settings {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 200px 1fr;
  }
  .pages {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    min-height: 0;
    overflow-y: auto;
    padding: var(--s-3) var(--s-2);
    border-right: 1px solid var(--faint);
    background: var(--lightbox);
  }
  .pages ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .pages button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 30px;
    padding: var(--s-1) var(--s-2);
    border-radius: var(--r-2);
    text-align: left;
    color: var(--ink);
  }
  .pages button :global(svg) {
    color: var(--muted);
  }
  .pages button:hover {
    background: var(--pressed);
  }
  .pages button.current {
    background: var(--sheet);
    font-weight: 500;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.06);
  }
  .pages button.current :global(svg) {
    color: var(--cyan);
  }
  .results button {
    flex-direction: column;
    align-items: flex-start;
    gap: 0;
  }
  .where {
    font-size: var(--text-1);
    color: var(--muted);
  }
  .none {
    padding: var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }

  .body {
    min-height: 0;
    overflow-y: auto;
    padding: var(--s-5) var(--s-6) var(--s-7);
  }
  .page-title {
    font-size: var(--text-4);
    margin: 0 0 var(--s-3);
  }
  .setting {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-5);
    padding: var(--s-4) 0;
    border-bottom: 1px solid var(--faint);
  }
  .setting .text {
    min-width: 0;
    max-width: 52ch;
  }
  .notes {
    padding: var(--s-3) var(--s-4);
    margin: var(--s-3) 0;
    border-radius: var(--r-2);
    background: var(--hover);
    white-space: pre-line;
  }
  .notes p {
    margin: 0 0 var(--s-1);
  }
  .error-text {
    color: var(--red);
  }
  .setting p {
    margin: 0;
  }
  .setting :global(.fm-select-trigger) {
    min-width: 180px;
    justify-content: space-between;
  }
  /* A setting found by search glows briefly where it landed. */
  .flash {
    animation: flash 1.2s var(--ease);
    border-radius: var(--r-2);
  }
  @keyframes flash {
    0%,
    40% {
      background: var(--selected);
      box-shadow: 0 0 0 6px var(--selected);
    }
  }
  .note {
    margin: var(--s-3) 0 0;
  }
  .intro {
    margin: 0 0 var(--s-4);
  }
  code {
    font-family: var(--mono);
    font-size: 0.95em;
  }
  .imported {
    list-style: none;
    margin: var(--s-3) 0 0;
    padding: 0;
  }
  .imported li {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    min-height: 36px;
    border-bottom: 1px solid var(--faint);
  }
  .imported .fam {
    font-weight: 500;
  }
  .imported .help {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  p {
    margin: 0 0 var(--s-3);
    line-height: 1.55;
  }
  a {
    color: var(--cyan);
    text-underline-offset: 2px;
  }
  a :global(svg) {
    display: inline;
    vertical-align: -1px;
    margin-left: 2px;
  }
  .more {
    color: var(--muted);
  }

  .label {
    display: block;
    font-weight: var(--weight-strong);
    margin: 0 0 var(--s-1);
  }
  .help {
    color: var(--muted);
    font-size: var(--text-1);
  }

  .steps {
    margin: 0 0 var(--s-5);
    padding-left: 1.4em;
    line-height: 1.55;
  }
  .steps li {
    margin-bottom: var(--s-2);
    padding-left: var(--s-1);
  }
  .steps li::marker {
    color: var(--muted);
  }

  .card {
    border: 1px solid var(--faint);
    border-radius: var(--r-3);
    padding: var(--s-4);
    margin-bottom: var(--s-5);
    background: var(--sheet);
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--s-2);
  }
  .field input {
    flex: 1;
    height: var(--control);
    padding: 0 var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    font-family: var(--mono);
    font-size: var(--text-2);
  }
  .field input:focus {
    outline: none;
    border-color: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
  }
  .field + .help {
    margin: var(--s-2) 0 var(--s-4);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
  }
  .saved {
    display: flex;
    gap: var(--s-3);
    margin-bottom: var(--s-4);
  }
  .saved p {
    margin: 0;
  }
  .key-icon {
    display: grid;
    place-items: center;
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: var(--r-full);
    background: var(--cyan-soft);
    color: var(--cyan);
  }
  .message {
    margin-top: var(--s-4);
  }

  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--s-2) var(--s-4);
    margin: 0 0 var(--s-4);
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .path {
    font-size: var(--text-1);
    color: var(--muted);
    font-family: var(--mono);
  }
</style>
