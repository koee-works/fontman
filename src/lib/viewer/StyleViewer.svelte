<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import { onMount } from 'svelte';
  import { api, type FamilySummary, type Pairing } from '../api';
  import Button from '../ui/Button.svelte';
  import Callout from '../ui/Callout.svelte';
  import Checkbox from '../ui/Checkbox.svelte';
  import ConfirmDialog from '../ui/ConfirmDialog.svelte';
  import FontPicker from '../ui/FontPicker.svelte';
  import Icon from '../ui/Icon.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import Menu from '../ui/Menu.svelte';
  import PromptDialog from '../ui/PromptDialog.svelte';
  import Select from '../ui/Select.svelte';
  import Slider from '../ui/Slider.svelte';
  import NumberField from '../ui/NumberField.svelte';
  import { fileStem, toCss, toJson, type FaceInfo } from './export';
  import { parsePairing, RATIOS, STYLES, sizeFor, type Role, type StyleKey } from './model';
  import Page from './Page.svelte';
  import { locatePairing, viewer } from './state.svelte';

  interface Props {
    families: FamilySummary[];
    /** Used when a saved font no longer exists: the family open in the inspector. */
    fallback?: FamilySummary;
  }
  let { families, fallback }: Props = $props();

  const s = $derived(viewer.s);

  $effect(() => {
    if (families.length) viewer.resolve(families, fallback);
  });

  // Persist every change, including slider drags that mutate settings directly.
  $effect(() => {
    JSON.stringify(viewer.s);
    viewer.save();
  });

  let notice = $state<{ tone: 'info' | 'warning' | 'error'; text: string } | null>(null);

  onMount(() => {
    viewer.loadPairings().catch((e) => (notice = { tone: 'error', text: `Couldn't load saved pairings: ${e}` }));
  });

  // Face lookups for both roles.
  const byFace = $derived.by(() => {
    const m = new Map<string, { family: FamilySummary; info: FaceInfo; variation: string | null }>();
    for (const fam of families)
      for (const f of fam.faces)
        m.set(f.id, {
          family: fam,
          variation: f.variation,
          info: {
            family: fam.name,
            subfamily: f.subfamily,
            weight: f.weight,
            italic: f.italic,
            postscriptName: f.postscriptName,
          },
        });
    return m;
  });
  const variations = $derived(new Map([...byFace].map(([id, v]) => [id, v.variation])));
  const familyOf = (role: Role) => (s[role] ? byFace.get(s[role]!.faceId)?.family : undefined);
  const faceItems = (role: Role) =>
    familyOf(role)?.faces.map((f) => ({ value: f.id, label: f.subfamily })) ?? [];

  const ratio = $derived(Number(s.ratio));
  const def = (key: StyleKey) => STYLES.find((d) => d.key === key)!;
  const px = (key: StyleKey) => sizeFor(def(key).step, s.base, ratio);
  const visibleCount = $derived(STYLES.filter((d) => s.styles[d.key].on).length);

  const round = (n: number) => (n >= 10 ? Math.round(n) : Math.round(n * 10) / 10);
  const roles = [
    { value: 'heading', label: 'Heading font' },
    { value: 'body', label: 'Body font' },
  ];

  // -- Saved pairings --------------------------------------------------------

  const current = $derived(viewer.pairing);
  const edited = $derived(viewer.edited);
  const title = $derived(current?.name ?? 'Unsaved pairing');

  /** A name for a new pairing: its two families. */
  const suggestedName = $derived(
    [s.heading?.family, s.body?.family]
      .filter((n, i, all): n is string => !!n && all.indexOf(n) === i)
      .join(' and ') || 'New pairing',
  );

  let prompt = $state<{
    title: string;
    confirmLabel: string;
    value: string;
    description?: string;
    submit: (v: string) => Promise<string | null>;
  } | null>(null);
  let promptOpen = $state(false);
  let deleting = $state<Pairing | null>(null);
  let deleteOpen = $state(false);

  function saveAsNew() {
    prompt = {
      title: 'Save pairing',
      confirmLabel: 'Save pairing',
      value: current && edited ? `${current.name} (2)` : suggestedName,
      description: 'Saves the fonts, scale and styles. The sample text stays shared between pairings.',
      submit: async (name) => {
        try {
          const p = await viewer.savePairing(name, true);
          notice = { tone: 'info', text: `Saved ${p.name}.` };
          return null;
        } catch (e) {
          return String(e);
        }
      },
    };
    promptOpen = true;
  }

  async function saveChanges() {
    if (!current) return saveAsNew();
    try {
      const p = await viewer.savePairing(current.name, false);
      notice = { tone: 'info', text: `Saved changes to ${p.name}.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    }
  }

  function rename(p: Pairing) {
    prompt = {
      title: `Rename ${p.name}`,
      confirmLabel: 'Rename',
      value: p.name,
      submit: async (name) => {
        try {
          await viewer.renamePairing(p, name);
          return null;
        } catch (e) {
          return String(e);
        }
      },
    };
    promptOpen = true;
  }

  function open(p: Pairing) {
    const missing = viewer.openPairing(p, families);
    notice = missing.length
      ? {
          tone: 'warning',
          text: `${missing.join(' and ')} ${missing.length === 1 ? 'is' : 'are'} no longer in your library, so ${p.name} uses another font in ${missing.length === 1 ? 'its place' : 'their place'}.`,
        }
      : null;
  }

  async function remove() {
    const p = deleting;
    if (!p) return;
    try {
      await viewer.deletePairing(p);
      if (compareId === p.id) compareId = null;
      notice = { tone: 'info', text: `Deleted ${p.name}.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      deleting = null;
    }
  }

  // -- Export ------------------------------------------------------------------

  const exportName = $derived(current?.name ?? suggestedName);
  const exportFaces = $derived({
    heading: s.heading ? byFace.get(s.heading.faceId)?.info : undefined,
    body: s.body ? byFace.get(s.body.faceId)?.info : undefined,
  });

  async function copyCss() {
    const css = toCss(exportName, s, exportFaces);
    try {
      await navigator.clipboard.writeText(css);
    } catch {
      // Older webviews: fall back to a hidden selection.
      const area = document.createElement('textarea');
      area.value = css;
      area.style.position = 'fixed';
      area.style.opacity = '0';
      document.body.append(area);
      area.select();
      const ok = document.execCommand('copy');
      area.remove();
      if (!ok) {
        notice = { tone: 'error', text: "Couldn't copy to the clipboard. Save the CSS as a file instead." };
        return;
      }
    }
    notice = { tone: 'info', text: 'Copied the CSS for this pairing.' };
  }

  async function saveFile(kind: 'css' | 'json') {
    const contents = kind === 'css' ? toCss(exportName, s, exportFaces) : toJson(exportName, s, exportFaces);
    try {
      const path = await api.saveTextFile(kind, `${fileStem(exportName)}.${kind}`, contents);
      if (path) notice = { tone: 'info', text: `Saved ${path}.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    }
  }

  // -- Compare -----------------------------------------------------------------

  let comparing = $state(false);
  let compareId = $state<number | null>(null);
  /** Pairings to compare against: any saved one other than the page's own, unedited. */
  const others = $derived(viewer.pairings.filter((p) => p.id !== current?.id || edited));
  const compareWith = $derived(others.find((p) => p.id === compareId) ?? others[0]);
  const compareSettings = $derived(
    compareWith ? locatePairing(parsePairing(compareWith.settings), families) : null,
  );

  // Scroll the two pages together, in proportion, so the same block sits
  // side by side even when one pairing sets everything larger.
  let left = $state<HTMLDivElement>();
  let right = $state<HTMLDivElement>();
  let syncing: HTMLElement | null = null;
  function sync(from: HTMLElement | undefined, to: HTMLElement | undefined) {
    if (!from || !to || (syncing && syncing !== from)) return;
    syncing = from;
    const max = from.scrollHeight - from.clientHeight;
    to.scrollTop = max > 0 ? (from.scrollTop / max) * (to.scrollHeight - to.clientHeight) : 0;
    requestAnimationFrame(() => (syncing = null));
  }
</script>

<div class="viewer">
  <section class="page-wrap" aria-label="Preview">
    <div class="toolbar">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger class="pairing-btn" aria-label="Pairing: {title}{edited ? ', edited' : ''}">
          <span class="pairing-name">{title}</span>
          {#if edited}<span class="edited">Edited</span>{/if}
          <Icon name="chevron-down" size={12} />
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content class="fm-menu pairing-menu" sideOffset={4} align="start">
            <DropdownMenu.Group>
              <DropdownMenu.GroupHeading class="fm-menu-heading">Saved pairings</DropdownMenu.GroupHeading>
              {#each viewer.pairings as p (p.id)}
                <DropdownMenu.Item class="fm-menu-item" onSelect={() => open(p)}>
                  <span class="fm-menu-check">{#if p.id === current?.id}<Icon name="check" size={14} />{/if}</span>
                  <span class="menu-name">{p.name}</span>
                </DropdownMenu.Item>
              {:else}
                <DropdownMenu.Item class="fm-menu-item" disabled>
                  <span class="fm-menu-check"></span>None yet
                </DropdownMenu.Item>
              {/each}
            </DropdownMenu.Group>
            <DropdownMenu.Separator class="fm-menu-separator" />
            <DropdownMenu.Item class="fm-menu-item" onSelect={saveAsNew}>
              <span class="fm-menu-check"><Icon name="save" size={14} /></span>Save as new pairing…
            </DropdownMenu.Item>
            {#if current}
              <DropdownMenu.Item class="fm-menu-item" onSelect={() => rename(current)}>
                <span class="fm-menu-check"><Icon name="edit" size={14} /></span>Rename…
              </DropdownMenu.Item>
              <DropdownMenu.Separator class="fm-menu-separator" />
              <DropdownMenu.Item
                class="fm-menu-item danger"
                onSelect={() => {
                  deleting = current;
                  deleteOpen = true;
                }}
              >
                <span class="fm-menu-check"><Icon name="trash" size={14} /></span>Delete pairing
              </DropdownMenu.Item>
            {/if}
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>

      {#if !current}
        <Button size="sm" variant="soft" tone="accent" icon="save" onclick={saveAsNew}>Save pairing…</Button>
      {:else if edited}
        <Button size="sm" variant="solid" tone="accent" icon="save" onclick={saveChanges}>Save changes</Button>
      {/if}

      <span class="spacer"></span>

      <IconButton
        icon="compare"
        size="sm"
        label={others.length ? 'Compare with a saved pairing' : 'Save a pairing to compare with it'}
        pressed={comparing}
        disabled={!others.length && !comparing}
        onclick={() => (comparing = !comparing)}
      />
      <Menu
        label="Export this pairing"
        text="Export"
        icon="export"
        items={[
          { label: 'Copy CSS', icon: 'copy', onSelect: copyCss },
          { label: 'Save CSS file…', icon: 'file-css', onSelect: () => saveFile('css') },
          { label: 'Save JSON file…', icon: 'file-json', onSelect: () => saveFile('json') },
        ]}
      />
    </div>

    {#if notice}
      <Callout tone={notice.tone} onDismiss={() => (notice = null)}>{notice.text}</Callout>
    {/if}

    <div class="pages" class:split={comparing}>
      <div class="column">
        {#if comparing}
          <div class="column-head">
            <span class="column-name">{title}{edited ? ' (edited)' : ''}</span>
            <span class="column-note">Editable</span>
          </div>
        {/if}
        <div class="scroll" bind:this={left} onscroll={() => comparing && sync(left, right)}>
          {#key viewer.textVersion}
            <Page settings={s} texts={s.texts} {variations} editable />
          {/key}
        </div>
      </div>

      {#if comparing}
        <div class="column">
          <div class="column-head">
            {#if others.length && compareWith}
              <Select
                size="sm"
                label="Pairing to compare with"
                items={others.map((p) => ({ value: String(p.id), label: p.name }))}
                value={String(compareWith.id)}
                onValueChange={(v) => (compareId = Number(v))}
              />
              <Button size="sm" variant="ghost" onclick={() => open(compareWith)}>Open</Button>
            {:else}
              <span class="column-name">No other saved pairing</span>
            {/if}
          </div>
          <div class="scroll" bind:this={right} onscroll={() => sync(right, left)}>
            {#if compareSettings}
              {#key viewer.textVersion}
                <Page settings={compareSettings} texts={s.texts} {variations} editable={false} />
              {/key}
            {:else}
              <p class="empty">Save a pairing, change the fonts or scale, then compare the two here.</p>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </section>

  <aside class="panel" aria-label="Style settings">
    <section aria-labelledby="fonts-heading">
      <h2 id="fonts-heading">Fonts</h2>
      {#each [{ role: 'heading', label: 'Heading font' }, { role: 'body', label: 'Body font' }] as r (r.role)}
        {@const role = r.role as Role}
        <div class="field">
          <span class="field-label">{r.label}</span>
          <!-- Remounted when the font changes, so the field always shows it. -->
          {#key s[role]?.family}
            <FontPicker
              {families}
              value={s[role]?.family ?? ''}
              label={r.label}
              onPick={(fam) => viewer.setFamily(role, fam)}
            />
          {/key}
          {#if faceItems(role).length > 1}
            <Select
              size="sm"
              label="{r.label} style"
              items={faceItems(role)}
              value={s[role]?.faceId ?? ''}
              onValueChange={(id) => s[role] && viewer.setFont(role, { family: s[role]!.family, faceId: id })}
            />
          {/if}
        </div>
      {/each}
    </section>

    <section aria-labelledby="scale-heading">
      <h2 id="scale-heading">Scale</h2>
      <div class="field">
        <span class="field-label">Ratio</span>
        <Select
          label="Scale ratio"
          items={RATIOS}
          value={s.ratio}
          onValueChange={(v) => {
            viewer.s.ratio = v;
            viewer.save();
          }}
        />
      </div>
      <div class="slider-row">
        <span class="field-label">Base size</span>
        <Slider bind:value={viewer.s.base} min={12} max={24} label="Base size" />
        <NumberField bind:value={viewer.s.base} min={12} max={24} unit="px" label="Base size" />
      </div>
      <div class="slider-row">
        <span class="field-label">Heading leading</span>
        <Slider bind:value={viewer.s.headingLeading} min={0.9} max={1.6} step={0.05} label="Heading line height" />
        <NumberField bind:value={viewer.s.headingLeading} min={0.9} max={1.6} step={0.05} decimals={2} label="Heading line height" />
      </div>
      <div class="slider-row">
        <span class="field-label">Body leading</span>
        <Slider bind:value={viewer.s.bodyLeading} min={1.1} max={2.2} step={0.05} label="Body line height" />
        <NumberField bind:value={viewer.s.bodyLeading} min={1.1} max={2.2} step={0.05} decimals={2} label="Body line height" />
      </div>
      <div class="slider-row">
        <span class="field-label">Line length</span>
        <Slider bind:value={viewer.s.measure} min={40} max={100} label="Line length in characters" />
        <NumberField bind:value={viewer.s.measure} min={40} max={100} unit="ch" label="Line length in characters" />
      </div>
    </section>

    <section aria-labelledby="styles-heading">
      <div class="section-head">
        <h2 id="styles-heading">Styles</h2>
        <Button size="sm" variant="ghost" onclick={() => viewer.allStyles(visibleCount < STYLES.length)}>
          {visibleCount < STYLES.length ? 'Show all' : 'Hide all'}
        </Button>
      </div>
      <ul class="styles">
        {#each STYLES as d (d.key)}
          <li class:off={!s.styles[d.key].on}>
            <Checkbox
              checked={s.styles[d.key].on}
              label="Show {d.label}"
              onCheckedChange={(on) => {
                viewer.s.styles[d.key].on = on;
                viewer.save();
              }}
            />
            <span class="style-name">{d.label}</span>
            <span class="style-size">{round(px(d.key))}px</span>
            <Select
              size="sm"
              label="Font for {d.label}"
              items={roles}
              value={s.styles[d.key].role}
              onValueChange={(v) => {
                viewer.s.styles[d.key].role = v as Role;
                viewer.save();
              }}
            />
          </li>
        {/each}
      </ul>
    </section>

    <div class="panel-foot">
      <p class="hint">Click any text on the page to edit it.</p>
      <div class="resets">
        <Button size="sm" variant="ghost" icon="reset" onclick={() => viewer.resetText()}>Reset text</Button>
        <Button size="sm" variant="ghost" icon="reset" onclick={() => viewer.reset(true)}>
          Reset scale and styles
        </Button>
      </div>
    </div>
  </aside>
</div>

<ConfirmDialog
  bind:open={deleteOpen}
  title={deleting ? `Delete ${deleting.name}?` : ''}
  confirmLabel="Delete pairing"
  onConfirm={remove}
>
  The saved pairing goes. The fonts stay in your library, and the page keeps showing these settings until you change them.
</ConfirmDialog>

{#if prompt}
  <PromptDialog
    bind:open={promptOpen}
    title={prompt.title}
    label="Name"
    confirmLabel={prompt.confirmLabel}
    value={prompt.value}
    description={prompt.description}
    onSubmit={prompt.submit}
  />
{/if}

<style>
  .viewer {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* Page ----------------------------------------------------------------- */
  .page-wrap {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 52px;
    padding: 0 var(--s-4) 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  .spacer {
    flex: 1;
  }
  /* The pairing's name is the toolbar's title, and opens its menu. */
  :global(.pairing-btn) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    min-width: 0;
    max-width: 320px;
    height: var(--control);
    margin-left: calc(var(--s-2) * -1);
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    font-size: var(--text-3);
    font-weight: var(--weight-strong);
    color: var(--ink);
  }
  :global(.pairing-btn:hover),
  :global(.pairing-btn[data-state='open']) {
    background: var(--hover);
  }
  :global(.pairing-btn svg) {
    color: var(--muted);
  }
  .pairing-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Unsaved changes are a state, not a warning: muted, like other meta. */
  .edited {
    font-size: var(--text-1);
    font-weight: 400;
    color: var(--muted);
  }
  :global(.pairing-menu) {
    max-width: 300px;
    max-height: min(440px, var(--bits-dropdown-menu-content-available-height, 440px));
    overflow-y: auto;
  }
  .menu-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pages {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr;
  }
  .pages.split {
    grid-template-columns: 1fr 1fr;
  }
  .column {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .split .column + .column {
    border-left: 1px solid var(--faint);
  }
  .column-head {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 40px;
    padding: 0 var(--s-4) 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
    background: var(--lightbox);
  }
  .column-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .column-note {
    margin-left: auto;
    font-size: var(--text-1);
    color: var(--muted);
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    /* Pages narrow their margins when a column is narrow, as in a comparison. */
    container-type: inline-size;
  }
  @container (max-width: 640px) {
    .scroll :global(.page) {
      padding: var(--s-7) var(--s-5) var(--s-8);
    }
  }
  .empty {
    margin: var(--s-7) var(--s-5);
    color: var(--muted);
    max-width: 40ch;
  }

  /* Panel ---------------------------------------------------------------- */
  .panel {
    width: 320px;
    flex: none;
    overflow-y: auto;
    border-left: 1px solid var(--faint);
    padding: var(--s-4) var(--s-5) var(--s-5);
    background: var(--sheet);
  }
  .panel section {
    padding-bottom: var(--s-5);
    margin-bottom: var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  h2 {
    font-size: var(--text-2);
    font-weight: var(--weight-strong);
    margin: 0 0 var(--s-3);
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--s-2);
  }
  .section-head h2 {
    margin: 0;
  }
  .field {
    display: grid;
    gap: var(--s-2);
    margin-bottom: var(--s-4);
  }
  .field :global(.fm-select-trigger) {
    justify-self: start;
  }
  .field-label {
    color: var(--muted);
    font-size: var(--text-1);
  }
  .slider-row {
    display: grid;
    grid-template-columns: 96px 1fr 44px;
    align-items: center;
    gap: var(--s-3);
    margin-bottom: var(--s-3);
  }
  .slider-row :global(.fm-number) {
    color: var(--muted);
  }
  .styles {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  /* Fixed columns, so sizes and font menus line up down the list whatever
     the label lengths. */
  .styles li {
    display: grid;
    grid-template-columns: 18px 1fr 44px 116px;
    align-items: center;
    gap: var(--s-2);
    min-height: 32px;
  }
  .styles li.off .style-name,
  .styles li.off .style-size {
    color: var(--muted);
  }
  .style-size {
    font-size: var(--text-1);
    color: var(--muted);
    text-align: right;
  }
  .styles li :global(.fm-select-trigger) {
    width: 100%;
    justify-content: space-between;
  }
  .panel-foot .hint {
    margin: 0 0 var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .resets {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-1);
    margin-left: calc(var(--s-2) * -1);
  }

  @media (max-width: 1100px) {
    .panel {
      width: 280px;
    }
  }
</style>
