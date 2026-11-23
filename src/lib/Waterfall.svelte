<script lang="ts">
  import { specimen } from './fonts';
  import Segmented from './ui/Segmented.svelte';

  // One line of the same text at a run of sizes, largest first: shows how a
  // face holds up from display down to the smallest text in one glance.
  interface Props {
    faceId: string;
    text: string;
    variation: string;
    /** CSS font-feature-settings from the Features tab. */
    features?: string;
  }
  let { faceId, text, variation, features = 'normal' }: Props = $props();

  const SIZES = [72, 60, 48, 40, 32, 28, 24, 20, 18, 16, 14, 13, 12, 11, 10, 9];

  // One line per size (the classic waterfall) or wrapped, to read every word
  // at large sizes. Remembered between launches.
  type Mode = 'line' | 'wrap';
  let mode = $state<Mode>(
    (() => {
      try {
        return localStorage.getItem('waterfall-mode') === 'wrap' ? 'wrap' : 'line';
      } catch {
        return 'line';
      }
    })(),
  );
  function setMode(m: Mode) {
    mode = m;
    try {
      localStorage.setItem('waterfall-mode', m);
    } catch {
      /* not persisted */
    }
  }
</script>

<div class="waterfall">
  <div class="head">
    <Segmented
      label="Line wrapping"
      items={[
        { value: 'line', label: 'One line' },
        { value: 'wrap', label: 'Wrap' },
      ]}
      value={mode}
      onValueChange={setMode}
    />
  </div>
  <ol class="sizes" class:wrap={mode === 'wrap'}>
    {#each SIZES as px (px)}
      <li>
        <span class="px">{px}</span>
        <span
          class="line"
          style:font-size="{px}px"
          style:font-variation-settings={variation}
          style:font-feature-settings={features}
          use:specimen={faceId}>{text}</span
        >
      </li>
    {/each}
  </ol>
</div>

<style>
  /* Scrolls to the panel edge, like the specimen tab. */
  .waterfall {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin-right: calc(var(--s-5) * -1);
    padding: var(--s-4) var(--s-5) var(--s-6) 0;
  }
  .head {
    margin-bottom: var(--s-3);
  }
  .sizes {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: grid;
    grid-template-columns: 24px 1fr;
    align-items: baseline;
    gap: var(--s-3);
    padding: var(--s-1) 0;
    border-bottom: 1px solid var(--faint);
  }
  li:last-child {
    border-bottom: 0;
  }
  .px {
    font-size: var(--text-1);
    color: var(--muted);
    text-align: right;
  }
  .line {
    min-width: 0;
    line-height: 1.25;
    font-synthesis: none;
    white-space: nowrap;
    overflow: hidden;
    /* A cut-off line fades out rather than ending mid-glyph. */
    mask-image: linear-gradient(to right, #000 calc(100% - 32px), transparent);
  }
  .wrap .line {
    white-space: normal;
    overflow-wrap: anywhere;
    mask-image: none;
  }
</style>
