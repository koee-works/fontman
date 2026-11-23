<script lang="ts">
  import { fontStack, specimen } from '../fonts';
  import {
    DEFAULT_BLOCKS,
    STYLES,
    sizeFor,
    type PairingSettings,
    type StyleKey,
  } from './model';
  import { viewer } from './state.svelte';

  // One sample page set in a pairing. The editable page writes its text back
  // to the viewer; a comparison page shows the same text, read-only.
  interface Props {
    settings: PairingSettings;
    texts: Record<string, string>;
    /** Variation settings by face id, pinning variable faces to their default. */
    variations: Map<string, string | null>;
    editable: boolean;
  }
  let { settings: s, texts, variations, editable }: Props = $props();

  const ratio = $derived(Number(s.ratio));
  const def = (key: StyleKey) => STYLES.find((d) => d.key === key)!;
  const px = (key: StyleKey) => sizeFor(def(key).step, s.base, ratio);
  const faceFor = (key: StyleKey) => s[s.styles[key].role]?.faceId ?? '';
  const variationFor = (key: StyleKey) => variations.get(faceFor(key)) ?? 'normal';
  const leadingFor = (key: StyleKey) =>
    def(key).leading === 'heading' ? s.headingLeading : s.bodyLeading;

  const visibleCount = $derived(STYLES.filter((d) => s.styles[d.key].on).length);
  const edit = $derived(editable ? 'plaintext-only' : undefined);

  const clean = (el: HTMLElement) => (el.innerText ?? '').replace(/\n+$/, '');

  function commit(id: string, el: HTMLElement) {
    if (!editable) return;
    const text = clean(el);
    if (text !== viewer.s.texts[id]) {
      viewer.s.texts[id] = text;
      viewer.save();
    }
  }
  /** Save as you type, so closing the app mid-edit loses nothing. */
  function draft(id: string, el: HTMLElement) {
    if (editable) viewer.saveDraft(id, clean(el));
  }
  function listText(id: string, index: number, el: HTMLElement) {
    const items = (viewer.s.texts[id] ?? '').split('\n');
    items[index] = el.innerText.replace(/\n/g, ' ').trim();
    return items.filter((i) => i.length).join('\n');
  }
  function draftListItem(id: string, index: number, el: HTMLElement) {
    if (editable) viewer.saveDraft(id, listText(id, index, el));
  }
  function commitListItem(id: string, index: number, el: HTMLElement) {
    if (!editable) return;
    viewer.s.texts[id] = listText(id, index, el);
    viewer.save();
  }

  const tags: Partial<Record<StyleKey, string>> = { h1: 'h1', h2: 'h2', h3: 'h3', h4: 'h4', lead: 'p', p: 'p' };
</script>

<article
  class="page"
  class:readonly={!editable}
  style:--measure="{s.measure}ch"
  style:font-size="{s.base}px"
  style:font-family={s.body ? fontStack(s.body.faceId) : undefined}
>
  {#if visibleCount === 0}
    <p class="nothing">All styles are hidden. Turn some on in the panel.</p>
  {/if}
  {#each DEFAULT_BLOCKS as block (block.id)}
    {@const key = block.style}
    {#if s.styles[key].on}
      {#if key === 'list'}
        <ul
          class="block s-list"
          style:font-size="{px(key)}px"
          style:line-height={leadingFor(key)}
          style:font-variation-settings={variationFor(key)}
          use:specimen={faceFor(key)}
        >
          {#each (texts[block.id] ?? '').split('\n') as item, i (i)}
            <li contenteditable={edit} spellcheck="false" oninput={(e) => draftListItem(block.id, i, e.currentTarget)} onblur={(e) => commitListItem(block.id, i, e.currentTarget)}>{item}</li>
          {/each}
        </ul>
      {:else if key === 'callout'}
        <aside
          class="block s-callout"
          style:font-size="{px(key)}px"
          style:line-height={leadingFor(key)}
          style:font-variation-settings={variationFor(key)}
          use:specimen={faceFor(key)}
          contenteditable={edit}
          spellcheck="false"
          oninput={(e) => draft(block.id, e.currentTarget)}
          onblur={(e) => commit(block.id, e.currentTarget)}
        >{texts[block.id]}</aside>
      {:else if key === 'quote'}
        <blockquote
          class="block s-quote"
          style:font-size="{px(key)}px"
          style:line-height={leadingFor(key)}
          style:font-variation-settings={variationFor(key)}
          use:specimen={faceFor(key)}
          contenteditable={edit}
          spellcheck="false"
          oninput={(e) => draft(block.id, e.currentTarget)}
          onblur={(e) => commit(block.id, e.currentTarget)}
        >{texts[block.id]}</blockquote>
      {:else if key === 'caption'}
        <figure class="block s-figure">
          <div class="figure-art" aria-hidden="true"></div>
          <figcaption
            class="s-caption"
            style:font-size="{px(key)}px"
            style:line-height={leadingFor(key)}
            style:font-variation-settings={variationFor(key)}
            use:specimen={faceFor(key)}
            contenteditable={edit}
            spellcheck="false"
            oninput={(e) => draft(block.id, e.currentTarget)}
            onblur={(e) => commit(block.id, e.currentTarget)}
          >{texts[block.id]}</figcaption>
        </figure>
      {:else}
        <svelte:element
          this={tags[key] ?? 'p'}
          class="block s-{key}"
          style:font-size="{px(key)}px"
          style:line-height={leadingFor(key)}
          style:font-variation-settings={variationFor(key)}
          use:specimen={faceFor(key)}
          contenteditable={edit}
          spellcheck="false"
          oninput={(e: Event) => draft(block.id, e.currentTarget as HTMLElement)}
          onblur={(e: FocusEvent) => commit(block.id, e.currentTarget as HTMLElement)}
        >{texts[block.id]}</svelte:element>
      {/if}
    {/if}
  {/each}
</article>

<style>
  /* The page column is as wide as the chosen line length of body text, in
     the body font's own characters, and centred like a printed page. */
  .page {
    max-width: var(--measure);
    margin: 0 auto;
    padding: var(--s-8) var(--s-7) var(--s-8);
    box-sizing: content-box;
    color: var(--ink);
  }
  .nothing {
    color: var(--muted);
    font: var(--text-2) var(--ui);
  }

  /* Spacing is in em, so the rhythm scales with the type. */
  .block {
    margin: 0 0 0.9em;
    font-synthesis: none;
    outline: none;
    border-radius: 2px;
    overflow-wrap: break-word;
  }
  .block:first-child {
    margin-top: 0;
  }
  [contenteditable]:hover {
    box-shadow: 0 0 0 4px var(--hover);
    background: var(--hover);
  }
  [contenteditable]:focus {
    box-shadow: 0 0 0 4px var(--selected);
    background: var(--selected);
  }
  .s-h1 { margin-bottom: 0.4em; }
  .s-h2 { margin: 1.5em 0 0.45em; }
  .s-h3 { margin: 1.4em 0 0.4em; }
  .s-h4 { margin: 1.3em 0 0.35em; }
  .s-lead { margin-bottom: 1.1em; }
  .s-list {
    padding-left: 1.3em;
  }
  .s-list li {
    margin-bottom: 0.3em;
    padding-left: 0.2em;
    outline: none;
  }
  .s-callout {
    margin: 1.3em 0;
    padding: 0.9em 1.1em;
    background: var(--hover);
    border-left: 3px solid var(--line);
    border-radius: 0 var(--r-2) var(--r-2) 0;
  }
  .s-figure {
    margin: 1.6em 0;
  }
  .figure-art {
    aspect-ratio: 16 / 7;
    border-radius: var(--r-2);
    /* The conventional crossed box: reads as "image goes here". */
    background:
      linear-gradient(to top right, transparent calc(50% - 0.5px), var(--line) 50%, transparent calc(50% + 0.5px)),
      linear-gradient(to bottom right, transparent calc(50% - 0.5px), var(--line) 50%, transparent calc(50% + 0.5px)),
      var(--hover);
  }
  .s-caption {
    margin-top: 0.7em;
    color: var(--muted);
    font-synthesis: none;
    outline: none;
  }
  .s-quote {
    margin: 1.5em 0;
    padding-left: 0.9em;
    border-left: 2px solid var(--ink);
  }

  @media (max-width: 1100px) {
    .page {
      padding: var(--s-7) var(--s-5) var(--s-8);
    }
  }
</style>
