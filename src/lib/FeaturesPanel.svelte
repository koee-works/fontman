<script lang="ts">
  import { api, type FontFeature } from './api';
  import { specimen } from './fonts';
  import { featureCss, GENERAL_SAMPLE, grouped, isOn, toggle, type FeatureView } from './features';
  import IconButton from './ui/IconButton.svelte';

  // The inspector's Features tab: every OpenType feature the face offers,
  // grouped, each a toggle. Pointing at a feature (or focusing it) shows text
  // that features it, off and on, in the preview at the top. Most features
  // only change particular characters, so the general sample would often
  // show nothing. What's switched on here applies to every inspector tab.
  interface Props {
    faceId: string;
    variation: string;
    /** The preview text typed above the list, if any: used for features
     *  without a sample of their own, and for the resting preview. */
    sample: string;
    /** Shown at rest when nothing is typed: the face's name. */
    title: string;
    /** Overrides of the browser's defaults, by tag. */
    on: Record<string, boolean>;
  }
  let { faceId, variation, sample, title, on = $bindable() }: Props = $props();

  let list = $state<FontFeature[] | null>(null);
  let error = $state<string | null>(null);
  $effect(() => {
    const id = faceId;
    list = null;
    error = null;
    api
      .faceFeatures(id)
      .then((l) => {
        if (id === faceId) list = l;
      })
      .catch((e) => {
        if (id === faceId) error = String(e);
      });
  });

  const groups = $derived(list ? grouped(list) : []);
  const views = $derived(new Map(groups.flatMap((g) => g.features).map((f) => [f.tag, f])));
  const changed = $derived(Object.keys(on).length);

  let focus = $state<FeatureView | null>(null);
  const textFor = (f: FeatureView) => f.sample ?? (sample.trim() || GENERAL_SAMPLE);
  const settingsWith = (tag: string, v: boolean) => featureCss({ ...on, [tag]: v });
</script>

<div class="panel">
  <div class="preview">
    <!-- The reset stays put whatever is previewed, so it's always reachable. -->
    <div class="preview-head">
      {#if focus}
        <span class="name">{focus.label}</span>
        <code>{focus.tag}</code>
      {:else}
        <span class="name">{changed ? 'With your changes' : 'As the font sets it'}</span>
      {/if}
      {#if changed}
        <IconButton icon="reset" size="sm" label="Reset features to the font's defaults" onclick={() => (on = {})} />
      {/if}
    </div>
    <div class="pair">
      {#if focus}
        <span class="state">Off</span>
        <p
          class="line"
          use:specimen={faceId}
          style:font-variation-settings={variation}
          style:font-feature-settings={settingsWith(focus.tag, false)}
        >
          {textFor(focus)}
        </p>
        <span class="state on">On</span>
        <p
          class="line"
          use:specimen={faceId}
          style:font-variation-settings={variation}
          style:font-feature-settings={settingsWith(focus.tag, true)}
        >
          {textFor(focus)}
        </p>
      {:else}
        <span></span>
        <p
          class="line"
          use:specimen={faceId}
          style:font-variation-settings={variation}
          style:font-feature-settings={featureCss(on)}
        >
          {sample.trim() || title}
        </p>
        <span></span>
        <p class="hint">Point at a feature to see it off and on.</p>
      {/if}
    </div>
  </div>

  {#if error}
    <p class="hint">Couldn't read this font's features: {error}</p>
  {:else if !list}
    <p class="hint">Reading features…</p>
  {:else if !groups.length}
    <p class="hint">This font has no OpenType features you can switch.</p>
  {:else}
    {#each groups as g (g.group)}
      <section aria-labelledby="feat-{g.group}">
        <h4 id="feat-{g.group}">{g.group}</h4>
        <div class="chips">
          {#each g.features as f (f.tag)}
            {@const pressed = isOn(on, f)}
            <button
              class="chip"
              aria-pressed={pressed}
              onclick={() => (on = toggle(on, f, !pressed, views))}
              onpointerenter={() => (focus = f)}
              onpointerleave={() => focus === f && (focus = null)}
              onfocus={() => (focus = f)}
              onblur={() => focus === f && (focus = null)}
            >
              <span class="label">{f.label}</span>
              <code>{f.tag}</code>
            </button>
          {/each}
        </div>
      </section>
    {/each}
  {/if}
</div>

<style>
  .panel {
    padding-bottom: var(--s-6);
  }
  /* Stays in view while the list of features scrolls under it. */
  /* One height whether resting or showing a feature, so the chips never move
     under the pointer: a heading row and two lines. */
  .preview {
    --line: clamp(24px, 5cqi, 44px);
    min-height: calc(var(--s-3) + 32px + 2 * var(--line) * 1.25 + var(--s-1) + var(--s-4) + 1px);
    box-sizing: border-box;
    position: sticky;
    top: 0;
    z-index: 1;
    margin: 0 0 var(--s-4);
    padding: var(--s-3) 0 var(--s-4);
    background: var(--sheet);
    border-bottom: 1px solid var(--faint);
  }
  .preview-head {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 24px;
    margin-bottom: var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .preview-head .name {
    color: var(--ink);
    font-weight: var(--weight-strong);
  }
  .preview-head :global(.fm-icon-btn) {
    margin-left: auto;
  }
  .pair {
    display: grid;
    grid-template-columns: 28px 1fr;
    align-items: baseline;
    gap: var(--s-1) var(--s-2);
  }
  .state {
    color: var(--muted);
    font-size: var(--text-1);
  }
  .state.on {
    color: var(--cyan);
  }
  .line {
    margin: 0;
    font-size: var(--line);
    line-height: 1.25;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-synthesis: none;
  }
  .hint {
    margin: 0;
    line-height: calc(var(--line) * 1.25);
    color: var(--muted);
    font-size: var(--text-1);
  }

  section {
    margin-bottom: var(--s-4);
  }
  h4 {
    margin: 0 0 var(--s-2);
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--muted);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-1);
  }
  /* A toggle: plain when off, the accent's tint when on. */
  .chip {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s-2);
    max-width: 100%;
    height: 26px;
    padding: 0 var(--s-2);
    border: 1px solid var(--faint);
    border-radius: var(--r-2);
    color: var(--ink);
    font-size: var(--text-1);
    line-height: 24px;
    transition:
      background-color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .chip:hover {
    background: var(--hover);
  }
  .chip[aria-pressed='true'] {
    background: var(--selected);
    border-color: color-mix(in srgb, var(--cyan-solid) 45%, transparent);
    color: var(--cyan);
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  code {
    font: var(--text-1) / 1 var(--mono, ui-monospace, monospace);
    color: var(--muted);
  }
  .chip[aria-pressed='true'] code {
    color: inherit;
    opacity: 0.75;
  }
</style>
