<script lang="ts">
  import { api, type FaceDetail, type FamilySummary } from './api';
  import { compare, type Pick, type Side } from './compare-state.svelte';
  import { specimen } from './fonts';
  import FontPicker from './ui/FontPicker.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import Segmented from './ui/Segmented.svelte';
  import Select from './ui/Select.svelte';
  import Slider from './ui/Slider.svelte';
  import NumberField from './ui/NumberField.svelte';

  // Two styles on the same text, in aligned rows, so differences show at a
  // glance: side by side, or one over the other. Details below mark what
  // differs.
  interface Props {
    families: FamilySummary[];
    /** The style open in the inspector, if any: the default left side. */
    preferred?: Pick;
  }
  let { families, preferred }: Props = $props();

  const s = $derived(compare.s);

  $effect(() => {
    if (families.length) compare.resolve(families, preferred);
  });
  // Persist slider drags, which change the size directly.
  $effect(() => {
    void compare.s.size;
    compare.save();
  });

  const byFace = $derived.by(() => {
    const m = new Map<string, { family: FamilySummary; variation: string | null; weight: number }>();
    for (const fam of families)
      for (const f of fam.faces) m.set(f.id, { family: fam, variation: f.variation, weight: f.weight });
    return m;
  });
  const famOf = (side: Side) => (s[side] ? byFace.get(s[side]!.faceId)?.family : undefined);
  const variationOf = (side: Side) => (s[side] ? byFace.get(s[side]!.faceId)?.variation : null) ?? 'normal';
  const faceIdOf = (side: Side) => s[side]?.faceId ?? '';

  // Full records, for the details table.
  let details = $state<Record<Side, FaceDetail | null>>({ left: null, right: null });
  $effect(() => {
    for (const side of ['left', 'right'] as Side[]) {
      const id = s[side]?.faceId;
      if (!id) {
        details[side] = null;
        continue;
      }
      if (details[side]?.faceId === id) continue;
      api
        .faceDetail(id)
        .then((d) => {
          if (compare.s[side]?.faceId === id) details[side] = d;
        })
        .catch(() => (details[side] = null));
    }
  });

  const label = (side: Side) => {
    const d = details[side];
    return d ? `${d.family} ${d.subfamily}` : (s[side]?.family ?? '');
  };

  const display = $derived(s.sample.trim() || 'Hamburgefonstiv');
  const sections = $derived([
    // `line`: one line per style, cut at the edge rather than wrapped, so
    // the two stay letter for letter comparable whatever their widths.
    { key: 'display', title: 'Your text', text: display, size: s.size, lh: 1.15, line: true, cell: s.size },
    { key: 'upper', title: 'Capitals', text: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', size: 32, lh: 1.3, line: true, cell: 56 },
    { key: 'lower', title: 'Lowercase', text: 'abcdefghijklmnopqrstuvwxyz', size: 32, lh: 1.3, line: true, cell: 56 },
    { key: 'figures', title: 'Figures and punctuation', text: '0123456789&?!@()[]“”‘’—–,.;:%$€£#', size: 28, lh: 1.4, line: true, cell: 56 },
    {
      key: 'text',
      title: 'Running text',
      text: 'Sphinx of black quartz, judge my vow. A typeface is judged in running text before it is judged in a headline: how evenly the page sits, how the figures 0123456789 line up, and whether the punctuation belongs with the letters around it.',
      size: 16,
      lh: 1.55,
      line: false,
      cell: 0,
    },
  ]);

  const widthName = ['', 'Ultra-condensed', 'Extra-condensed', 'Condensed', 'Semi-condensed', 'Normal', 'Semi-expanded', 'Expanded', 'Extra-expanded', 'Ultra-expanded'];
  const formatName: Record<string, string> = { truetype: 'TrueType', opentype: 'OpenType (CFF)', collection: 'Font collection', unsupported: 'Unsupported' };
  const facts: { label: string; value: (d: FaceDetail) => string }[] = [
    { label: 'Family', value: (d) => d.family },
    { label: 'Style', value: (d) => d.subfamily },
    { label: 'Weight', value: (d) => String(d.weight) },
    { label: 'Width', value: (d) => widthName[d.width] ?? String(d.width) },
    { label: 'Slope', value: (d) => (d.italic ? 'Italic' : d.oblique ? 'Oblique' : 'Upright') },
    { label: 'Format', value: (d) => `${formatName[d.format]}${d.isVariable ? ', variable' : ''}` },
    { label: 'Variable axes', value: (d) => (d.axes.length ? d.axes.map((a) => `${a.tag} ${a.min}–${a.max}`).join(', ') : 'None') },
    { label: 'Glyphs', value: (d) => d.glyphCount.toLocaleString() },
    { label: 'Characters', value: (d) => d.codepointCount.toLocaleString() },
    { label: 'Units per em', value: (d) => String(d.unitsPerEm) },
    { label: 'Version', value: (d) => d.version ?? 'Not given' },
    { label: 'PostScript name', value: (d) => d.postscriptName ?? 'None' },
    { label: 'Status', value: (d) => (d.installed ? 'Installed by the system' : d.active ? 'Active' : 'Not active') },
  ];
  const differing = $derived(
    details.left && details.right ? facts.filter((f) => f.value(details.left!) !== f.value(details.right!)).length : 0,
  );
</script>

{#snippet header(side: Side)}
  {@const fam = famOf(side)}
  <div class="pick">
    <span class="side-mark {side}" aria-hidden="true"></span>
    {#key s[side]?.family}
      <FontPicker
        {families}
        value={s[side]?.family ?? ''}
        label="{side === 'left' ? 'First' : 'Second'} font"
        onPick={(f) => compare.setFamily(side, f, byFace.get(faceIdOf(side === 'left' ? 'right' : 'left'))?.weight)}
      />
    {/key}
    {#if fam && fam.faces.length > 1}
      <Select
        size="sm"
        label="{side === 'left' ? 'First' : 'Second'} style"
        items={fam.faces.map((f) => ({ value: f.id, label: f.subfamily }))}
        value={faceIdOf(side)}
        onValueChange={(id) => compare.set(side, { family: fam.name, faceId: id })}
      />
    {/if}
  </div>
{/snippet}

{#snippet sample(side: Side, text: string, size: number, lh: number, line = false)}
  <p
    class="spec {side}"
    class:line
    style:font-size="{size}px"
    style:line-height={lh}
    style:font-variation-settings={variationOf(side)}
    use:specimen={faceIdOf(side)}
  >{text}</p>
{/snippet}

<div class="compare">
  <div class="toolbar">
    <label class="sample">
      <span class="visually-hidden">Sample text</span>
      <input
        type="text"
        placeholder="Type to compare. Empty shows Hamburgefonstiv."
        spellcheck="false"
        value={s.sample}
        oninput={(e) => {
          compare.s.sample = e.currentTarget.value;
          compare.save();
        }}
      />
    </label>
    <Segmented
      label="Layout"
      items={[
        { value: 'side', label: 'Side by side' },
        { value: 'overlay', label: 'Overlay' },
      ]}
      value={s.mode}
      onValueChange={(v) => {
        compare.s.mode = v;
        compare.save();
      }}
    />
    <div class="size">
      <Slider bind:value={compare.s.size} min={16} max={200} label="Size of your text" />
      <NumberField bind:value={compare.s.size} min={16} max={200} unit="px" label="Size of your text" />
    </div>
    <IconButton icon="swap" label="Swap the two styles" onclick={() => compare.swap()} />
  </div>

  <div class="scroll">
    <div class="heads" class:overlay={s.mode === 'overlay'}>
      {@render header('left')}
      {@render header('right')}
    </div>

    {#each sections as sec (sec.key)}
      <section aria-label={sec.title}>
        <h3>{sec.title}</h3>
        {#if s.mode === 'side' || !sec.cell}
          <div class="pair">
            {@render sample('left', sec.text, sec.size, sec.lh, sec.line)}
            {@render sample('right', sec.text, sec.size, sec.lh, sec.line)}
          </div>
        {:else}
          <!-- Letter by letter, one glyph over the other on a shared baseline:
               the first in ink, the second in the accent colour. Whole lines
               would drift apart as soon as the widths differ. -->
          <div class="cells" style:font-size="{sec.cell}px">
            {#each [...sec.text] as ch, i (i)}
              {#if ch.trim()}
                <span class="cell" aria-label={ch}>
                  <span class="glyph left" style:font-variation-settings={variationOf('left')} use:specimen={faceIdOf('left')}>{ch}</span>
                  <span class="glyph right" style:font-variation-settings={variationOf('right')} use:specimen={faceIdOf('right')}>{ch}</span>
                </span>
              {/if}
            {/each}
          </div>
        {/if}
      </section>
    {/each}

    <section class="details" aria-labelledby="compare-details">
      <h3 id="compare-details">
        Details{#if details.left && details.right}<span class="diff-count">
            {differing === 0 ? 'Identical' : `${differing} ${differing === 1 ? 'difference' : 'differences'}`}</span
          >{/if}
      </h3>
      {#if details.left && details.right}
        <table>
          <thead>
            <tr>
              <th scope="col"><span class="visually-hidden">Property</span></th>
              <th scope="col"><span class="side-mark left" aria-hidden="true"></span>{label('left')}</th>
              <th scope="col"><span class="side-mark right" aria-hidden="true"></span>{label('right')}</th>
            </tr>
          </thead>
          <tbody>
            {#each facts as f (f.label)}
              {@const a = f.value(details.left)}
              {@const b = f.value(details.right)}
              <tr class:differs={a !== b}>
                <th scope="row">
                  {#if a !== b}<span class="dot" aria-hidden="true"></span>{/if}{f.label}
                  {#if a !== b}<span class="visually-hidden">(differs)</span>{/if}
                </th>
                <td>{a}</td>
                <td>{b}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        <p class="muted">Loading details…</p>
      {/if}
    </section>
  </div>
</div>

<style>
  .compare {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-4);
    height: 52px;
    flex: none;
    padding: 0 var(--s-4) 0 var(--s-5);
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

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 var(--s-5) var(--s-7);
  }
  /* The pickers stay in view while the specimens scroll under them. */
  .heads {
    position: sticky;
    top: 0;
    z-index: 1;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-6);
    padding: var(--s-3) 0;
    background: var(--sheet);
    border-bottom: 1px solid var(--faint);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-width: 0;
  }
  .pick :global(.fm-combo) {
    flex: 1;
    min-width: 0;
    max-width: 320px;
  }
  /* Which side is which, everywhere: ink for the first, accent for the
     second, matching the overlay. */
  .side-mark {
    width: 10px;
    height: 10px;
    flex: none;
    border-radius: var(--r-full);
    display: inline-block;
  }
  .side-mark.left {
    background: var(--ink);
  }
  .side-mark.right {
    background: var(--cyan);
  }

  section {
    padding: var(--s-4) 0;
    border-bottom: 1px solid var(--faint);
  }
  h3 {
    margin: 0 0 var(--s-2);
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--muted);
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s-6);
    align-items: baseline;
  }
  .spec {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
    font-synthesis: none;
    color: var(--ink);
  }
  .spec.line {
    white-space: nowrap;
    overflow: hidden;
    overflow-wrap: normal;
    /* Cut lines fade out rather than end mid-glyph. */
    mask-image: linear-gradient(to right, #000 calc(100% - 40px), transparent);
  }

  /* Overlay: a cell per character, sized to the wider glyph. Both glyphs
     share the cell's one grid area, and baseline alignment puts them on the
     same baseline whatever each font's ascent. */
  .cells {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    line-height: 1.25;
  }
  .cell {
    display: grid;
    grid-template-areas: 'g';
    align-items: baseline;
    justify-items: center;
    min-width: 0.6em;
    padding: 0 0.08em;
    border-radius: var(--r-1);
    background: var(--hover);
  }
  .glyph {
    grid-area: g;
    font-synthesis: none;
    color: var(--ink);
  }
  .glyph.right {
    color: var(--cyan);
    opacity: 0.8;
  }

  .details table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-2);
  }
  .details th,
  .details td {
    text-align: left;
    padding: var(--s-2) var(--s-3) var(--s-2) 0;
    border-bottom: 1px solid var(--faint);
    vertical-align: top;
    overflow-wrap: anywhere;
  }
  .details thead th {
    font-weight: var(--weight-strong);
  }
  .details thead .side-mark {
    margin-right: var(--s-2);
  }
  .details tbody th {
    width: 22%;
    color: var(--muted);
    font-weight: 400;
    white-space: nowrap;
  }
  .details td {
    width: 39%;
  }
  .differs td {
    font-weight: var(--weight-strong);
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin: 0 var(--s-2) 2px 0;
    border-radius: var(--r-full);
    background: var(--cyan);
  }
  .diff-count {
    margin-left: var(--s-2);
    font-weight: 400;
  }
  .muted {
    color: var(--muted);
  }
</style>
