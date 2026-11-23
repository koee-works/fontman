<script lang="ts">
  import { Tabs as BitsTabs } from 'bits-ui';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import type { Collection, FaceDetail, Tag } from './api';
  import TagEditor from './ui/TagEditor.svelte';
  import CollectionMenu from './ui/CollectionMenu.svelte';
  import Waterfall from './Waterfall.svelte';
  import FeaturesPanel from './FeaturesPanel.svelte';
  import { featureCss } from './features';
  import { specimen } from './fonts';
  import GlyphMap from './GlyphMap.svelte';
  import Button from './ui/Button.svelte';
  import Callout from './ui/Callout.svelte';
  import Icon from './ui/Icon.svelte';
  import IconButton from './ui/IconButton.svelte';
  import Slider from './ui/Slider.svelte';
  import NumberField from './ui/NumberField.svelte';
  import Tabs from './ui/Tabs.svelte';
  import Select from './ui/Select.svelte';

  interface Props {
    face: FaceDetail;
    sample: string;
    conflict: boolean;
    busy: boolean;
    /** Every style of this face's family, for switching without the list. */
    styles: { value: string; label: string }[];
    onSelectStyle: (faceId: string) => void;
    /** Send this face to the style viewer as the heading or body font. */
    onTryInViewer: (role: 'heading' | 'body') => void;
    /** Open Compare with this style on the left. */
    onCompare: () => void;
    onToggle: (active: boolean) => void;
    onClose: () => void;
    collections: Collection[];
    /** Add this style to a collection, or take it out. */
    onCollection: (collection: Collection, member: boolean) => void;
    onNewCollection: () => void;
    /** The family's display name, which tags are attached to. */
    familyName: string;
    tags: Tag[];
    onTag: (tag: Tag, tagged: boolean) => void;
    onCreateTag: (name: string) => Promise<string | null>;
  }
  let {
    face,
    sample,
    conflict,
    busy,
    styles,
    onSelectStyle,
    onTryInViewer,
    onCompare,
    onToggle,
    onClose,
    collections,
    onCollection,
    onNewCollection,
    familyName,
    tags,
    onTag,
    onCreateTag,
  }: Props = $props();

  const memberOf = $derived(collections.filter((c) => c.faceIds.includes(face.faceId)));

  let tab = $state('specimen');

  // Axis positions, reset whenever a different face is opened.
  let axisValues = $state<Record<string, number>>({});
  $effect(() => {
    axisValues = Object.fromEntries(face.axes.map((a) => [a.tag, a.default]));
  });

  const variation = $derived(
    face.axes
      .filter((a) => axisValues[a.tag] !== undefined)
      .map((a) => `"${a.tag}" ${axisValues[a.tag]}`)
      .join(', ') || 'normal',
  );
  const visibleAxes = $derived(face.axes.filter((a) => !a.hidden));

  // OpenType features switched from the font's defaults, also reset per face.
  // They apply to every tab, so a feature can be judged in running text, at
  // every size and across the whole character set.
  let featureOn = $state<Record<string, boolean>>({});
  $effect(() => {
    void face.faceId;
    featureOn = {};
  });
  const features = $derived(featureCss(featureOn));
  const featureCount = $derived(Object.keys(featureOn).length);
  const changed = $derived(face.axes.some((a) => axisValues[a.tag] !== a.default));

  const heading = $derived(sample.trim() || face.fullName || face.family);
  const waterfallText = $derived(sample.trim() || 'Sphinx of black quartz, judge my vow');

  const formatName: Record<string, string> = {
    truetype: 'TrueType',
    opentype: 'OpenType (CFF)',
    collection: 'Font collection',
    unsupported: 'Unsupported',
  };
  const widthName = [
    '', 'Ultra-condensed', 'Extra-condensed', 'Condensed', 'Semi-condensed',
    'Normal', 'Semi-expanded', 'Expanded', 'Extra-expanded', 'Ultra-expanded',
  ];
  const size = (n: number) =>
    n > 1024 * 1024 ? `${(n / 1024 / 1024).toFixed(1)} MB` : `${Math.round(n / 1024)} KB`;
</script>

<aside class="inspector" aria-label="{face.family} {face.subfamily}">
  <header>
    <div class="title">
      <h2>{face.family}</h2>
      {#if styles.length > 1}
        <div class="style-picker">
          <Select
            size="sm"
            label="Style"
            items={styles}
            value={face.faceId}
            onValueChange={(id) => id !== face.faceId && onSelectStyle(id)}
          />
        </div>
      {:else}
        <p>{face.subfamily}</p>
      {/if}
    </div>
    <IconButton icon="compare" label="Compare with another style" onclick={onCompare} />
    <IconButton icon="x" label="Close inspector (Esc)" onclick={onClose} />
  </header>

  <div class="status">
    {#if face.installed}
      <span class="pill neutral"><Icon name="lock" size={12} />Installed by the system</span>
    {:else if face.active}
      <span class="pill on"><span class="dot"></span>Active</span>
      <Button size="sm" variant="outline" disabled={busy} onclick={() => onToggle(false)}>
        Deactivate
      </Button>
    {:else}
      <span class="pill neutral">Not active</span>
      <Button size="sm" variant="solid" tone="accent" disabled={busy} onclick={() => onToggle(true)}>
        Activate
      </Button>
    {/if}
  </div>

  <div class="try">
    <span>Try in style viewer</span>
    <Button size="sm" variant="soft" icon="heading" onclick={() => onTryInViewer('heading')}>As headings</Button>
    <Button size="sm" variant="soft" icon="body-text" onclick={() => onTryInViewer('body')}>As body text</Button>
  </div>

  <div class="try">
    <span class="in" title={memberOf.map((c) => c.name).join(', ')}>
      {#if memberOf.length}
        In {memberOf.map((c) => c.name).join(', ')}
      {:else}
        Not in a collection
      {/if}
    </span>
    <CollectionMenu
      {collections}
      faceIds={[face.faceId]}
      label="Add {face.family} {face.subfamily} to a collection"
      text="Collections"
      onToggle={onCollection}
      onCreate={onNewCollection}
    />
  </div>

  <div class="tag-row">
    <TagEditor
      family={familyName}
      {tags}
      onAdd={(t) => onTag(t, true)}
      onRemove={(t) => onTag(t, false)}
      onCreate={onCreateTag}
    />
  </div>

  {#if conflict}
    <div class="conflict">
      <Callout tone="conflict" variant="box">
        Another active font is also named {face.postscriptName}. Apps will pick one of them
        unpredictably.
      </Callout>
    </div>
  {/if}

  <Tabs
    bind:value={tab}
    label="Inspector views"
    tabs={[
      { value: 'specimen', label: 'Specimen' },
      { value: 'waterfall', label: 'Waterfall' },
      { value: 'features', label: featureCount ? `Features (${featureCount})` : 'Features' },
      { value: 'characters', label: 'Characters' },
    ]}
  >
    <BitsTabs.Content value="specimen" class="fm-tab-panel">
      <div class="scroll">
        <p
          class="display"
          use:specimen={face.faceId}
          style:font-variation-settings={variation}
          style:font-feature-settings={features}
        >
          {heading}
        </p>
        <p
          class="text"
          use:specimen={face.faceId}
          style:font-variation-settings={variation}
          style:font-feature-settings={features}
        >
          Sphinx of black quartz, judge my vow. Typography is what language looks like, and a
          typeface is judged in running text before it is judged in a headline: how evenly the
          page sits, how the figures 0123456789 line up, whether an ampersand & a question
          mark? belong to the same family as the letters around them.
        </p>

        {#if visibleAxes.length}
          <section class="axes" aria-labelledby="axes-heading">
            <div class="section-head">
              <h3 id="axes-heading">Variable axes</h3>
              {#if changed}
                <IconButton
                  icon="reset"
                  size="sm"
                  label="Reset axes to default"
                  onclick={() =>
                    (axisValues = Object.fromEntries(face.axes.map((a) => [a.tag, a.default])))}
                />
              {/if}
            </div>
            {#each visibleAxes as axis (axis.tag)}
              <div class="axis">
                <span class="axis-name" title={axis.tag}>{axis.name ?? axis.tag}</span>
                <Slider
                  bind:value={axisValues[axis.tag]!}
                  min={axis.min}
                  max={axis.max}
                  step={axis.max - axis.min > 20 ? 1 : 0.1}
                  label={axis.name ?? axis.tag}
                />
                <NumberField
                  bind:value={axisValues[axis.tag]!}
                  min={axis.min}
                  max={axis.max}
                  step={axis.max - axis.min > 20 ? 1 : 0.1}
                  decimals={axis.max - axis.min > 20 ? 0 : 1}
                  label="{axis.name ?? axis.tag} value"
                />
              </div>
            {/each}
          </section>
        {/if}

        <section aria-labelledby="details-heading">
          <h3 id="details-heading">Details</h3>
          <dl class="facts">
            <dt>PostScript name</dt><dd>{face.postscriptName ?? 'None'}</dd>
            <dt>Weight</dt><dd>{face.weight}</dd>
            <dt>Width</dt><dd>{widthName[face.width] ?? face.width}</dd>
            <dt>Format</dt><dd>{formatName[face.format]}{face.isVariable ? ', variable' : ''}</dd>
            {#if face.version}<dt>Version</dt><dd>{face.version}</dd>{/if}
            <dt>Glyphs</dt><dd>{face.glyphCount.toLocaleString()}</dd>
            <dt>Characters</dt><dd>{face.codepointCount.toLocaleString()}</dd>
            {#if face.legacyFamily !== face.family}
              <dt>Older apps list it as</dt><dd>{face.legacyFamily} {face.legacySubfamily}</dd>
            {/if}
          </dl>
          <div class="file">
            <div class="path" title={face.filePath}>
              {face.filePath}{face.format === 'collection' ? `, face ${face.faceIndex + 1}` : ''}<span
                class="size">{size(face.fileSize)}</span
              >
            </div>
            <IconButton
              icon="reveal"
              size="sm"
              label="Show in folder"
              onclick={() => revealItemInDir(face.filePath).catch(() => {})}
            />
          </div>
        </section>
      </div>
    </BitsTabs.Content>
    <BitsTabs.Content value="waterfall" class="fm-tab-panel">
      <Waterfall faceId={face.faceId} text={waterfallText} {variation} {features} />
    </BitsTabs.Content>
    <BitsTabs.Content value="features" class="fm-tab-panel">
      <div class="scroll">
        <FeaturesPanel
          faceId={face.faceId}
          {variation}
          {sample}
          title={face.fullName || face.family}
          bind:on={featureOn}
        />
      </div>
    </BitsTabs.Content>
    <BitsTabs.Content value="characters" class="fm-tab-panel">
      <GlyphMap faceId={face.faceId} {variation} {features} />
    </BitsTabs.Content>
  </Tabs>
</aside>

<style>
  .inspector {
    /* Sizes below follow the inspector's width, so widening it gives the
       specimen room rather than just longer lines. */
    container-type: inline-size;
    background: var(--sheet);
    border-left: var(--pane-rule-width) solid var(--faint);
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: var(--s-4) var(--s-5) 0;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: var(--s-3);
  }
  .title {
    flex: 1;
    min-width: 0;
  }
  h2 {
    font-size: var(--text-4);
    margin: 0;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }
  .title p {
    margin: 0;
    color: var(--muted);
  }
  .style-picker {
    margin-top: var(--s-1);
  }

  .status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-2);
    margin: var(--s-3) 0;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    height: 24px;
    padding: 0 var(--s-3);
    border-radius: var(--r-full);
    font-size: var(--text-1);
    font-weight: 500;
  }
  .pill.on {
    background: var(--cyan-soft);
    color: var(--cyan);
  }
  .pill.neutral {
    background: var(--hover);
    color: var(--muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: var(--r-full);
    background: currentColor;
  }
  .conflict {
    margin-bottom: var(--s-3);
  }
  .try {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2);
    margin: 0 0 var(--s-3);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .try > span {
    margin-right: auto;
  }
  .tag-row {
    margin: calc(var(--s-1) * -1) 0 var(--s-3);
  }
  .in {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The scroll area runs to the panel's right edge, so its scrollbar sits
     there, with the same gutter restored inside for the text. */
  .scroll {
    overflow-y: auto;
    min-height: 0;
    flex: 1;
    margin-right: calc(var(--s-5) * -1);
    padding: 0 var(--s-5) var(--s-6) 0;
  }
  .display {
    font-size: clamp(56px, 10cqi, 144px);
    line-height: 1.1;
    margin: var(--s-5) 0 var(--s-4);
    overflow-wrap: anywhere;
    font-synthesis: none;
  }
  .text {
    font-size: clamp(16px, 2cqi, 20px);
    line-height: 1.55;
    /* A readable measure, however wide the inspector gets. */
    max-width: 68ch;
    margin: 0 0 var(--s-6);
    font-synthesis: none;
  }

  section {
    margin-bottom: var(--s-6);
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 24px;
    margin-bottom: var(--s-2);
  }
  h3 {
    font-size: var(--text-2);
    font-weight: var(--weight-strong);
    margin: 0 0 var(--s-2);
  }
  .section-head h3 {
    margin: 0;
  }
  .axis {
    display: grid;
    grid-template-columns: 96px 1fr 44px;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-1) 0;
  }
  .axis-name {
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--s-2) var(--s-4);
    margin: 0 0 var(--s-3);
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .file {
    display: flex;
    align-items: flex-start;
    gap: var(--s-2);
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-3);
    border-radius: var(--r-2);
    background: var(--hover);
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: var(--text-1);
    color: var(--muted);
    overflow-wrap: anywhere;
    line-height: 1.5;
  }
  .size {
    white-space: nowrap;
  }
  .size::before {
    content: ', ';
  }
</style>
