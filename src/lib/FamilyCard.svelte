<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FamilySummary } from './api';
  import { specimen } from './fonts';
  import { draggableFamily, type DragPayload } from './drag';
  import ActiveToggle from './ui/ActiveToggle.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import Icon from './ui/Icon.svelte';

  interface Props {
    family: FamilySummary;
    sample: string;
    /** Specimen size in px; the card height follows it. */
    size: number;
    /** Open in the inspector. */
    current: boolean;
    conflict: boolean;
    busy: boolean;
    /** Switch the family's library styles on or off. */
    onToggle: (family: FamilySummary, active: boolean) => void;
    onSelect: (faceId: string) => void;
    selectedFaces: Set<string>;
    onSelectFaces: (ids: string[], selected: boolean) => void;
    /** Extra controls in the footer, revealed on hover or focus. */
    actions?: Snippet<[FamilySummary]>;
    /** What dragging this card carries: the family, or the whole selection. */
    dragPayload: (family: FamilySummary) => DragPayload;
    /** Names of the family's tags. */
    tags?: string[];
  }
  let {
    family,
    sample,
    size,
    current,
    conflict,
    busy,
    onToggle,
    onSelect,
    selectedFaces,
    onSelectFaces,
    actions,
    dragPayload,
    tags = [],
  }: Props = $props();

  const text = $derived(sample.trim() || family.name);
  const toggleable = $derived(family.faces.filter((f) => !f.installed));
  const onCount = $derived(toggleable.filter((f) => f.active).length);
  const picked = $derived(family.faces.filter((f) => selectedFaces.has(f.id)).length);
  const variation = $derived(
    family.faces.find((f) => f.id === family.representative)?.variation ?? 'normal',
  );
  const meta = $derived(
    [
      family.faces.length === 1 ? '1 style' : `${family.faces.length} styles`,
      family.variable ? 'variable' : null,
      family.installed ? 'installed' : null,
    ]
      .filter(Boolean)
      .join(', '),
  );
</script>

<article
  class="card"
  data-family={family.name}
  class:current
  aria-label={family.name}
  use:draggableFamily={() => dragPayload(family)}
>
  <button
    class="specimen-area"
    onclick={() => onSelect(family.representative)}
    aria-label="Open {family.name} in the inspector"
  >
    <span
      class="specimen"
      style:font-size="{size}px"
      style:font-variation-settings={variation}
      use:specimen={family.representative}
    >
      {text}
    </span>
  </button>

  <footer>
    <span class="check">
      <Checkbox
        checked={picked > 0 && picked === family.faces.length}
        indeterminate={picked > 0 && picked < family.faces.length}
        label="Select {family.name}"
        onCheckedChange={(on) => onSelectFaces(family.faces.map((f) => f.id), on)}
      />
    </span>
    <ActiveToggle
      state={family.installed ? 'installed' : family.activation}
      name={family.name}
      on={onCount}
      total={toggleable.length}
      disabled={busy}
      onToggle={(on) => onToggle(family, on)}
    />
    <div class="info">
      <p class="name" title={family.name}>{family.name}</p>
      <p class="meta">
        {meta}{#if tags.length}<span class="card-tags"
            ><Icon name="tag" size={12} />{tags.join(', ')}</span
          >{/if}
      </p>
    </div>
    {#if conflict}
      <span class="badge" title="An active font elsewhere uses the same PostScript name">
        <Icon name="warning" size={12} label="Name conflict" />
      </span>
    {/if}
    {#if actions}<span class="actions">{@render actions(family)}</span>{/if}
  </footer>
</article>

<style>
  /* The border is always 1px so selecting a card never shifts the layout;
     selection only changes its colour, plus a ring drawn outside the card,
     where the grid gap leaves room for it. */
  .card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--sheet);
    border: 1px solid var(--faint);
    border-radius: var(--r-3);
    transition: border-color var(--dur-fast) var(--ease);
  }
  .card:hover {
    border-color: var(--line);
  }
  .card.current {
    border-color: var(--cyan-solid);
    box-shadow: 0 0 0 1px var(--cyan-solid);
  }

  .specimen-area {
    display: block;
    padding: var(--s-4) var(--s-4) var(--s-3);
    text-align: left;
    border-radius: var(--r-3) var(--r-3) 0 0;
    color: var(--ink);
  }
  .specimen-area:focus-visible {
    outline-offset: -2px;
  }
  /* Two lines, always: every card in a row is the same height whatever the
     text, and long samples end cleanly instead of spilling. */
  .specimen {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    line-height: 1.2;
    height: 2.4em;
    overflow-wrap: anywhere;
    font-synthesis: none;
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-4);
    border-top: 1px solid var(--faint);
  }
  .check {
    display: grid;
    place-items: center;
    width: 18px;
    flex: none;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .name,
  .meta {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    font-weight: var(--weight-strong);
  }
  .meta {
    color: var(--muted);
    font-size: var(--text-1);
  }
  .actions {
    display: flex;
    margin: -4px -4px -4px 0;
    opacity: 0;
  }
  .card:hover .actions,
  .card:focus-within .actions,
  .actions:has(:global([data-state='open'])),
  .actions:has(:global([data-member])) {
    opacity: 1;
  }
  @media (hover: none) {
    .actions {
      opacity: 1;
    }
  }
  /* Inline, not a flex box, so a long list truncates with the line's
     ellipsis instead of disappearing whole. */
  .card-tags {
    margin-left: var(--s-2);
  }
  .card-tags :global(svg) {
    display: inline-block;
    vertical-align: -2px;
    margin-right: 2px;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: var(--r-full);
    background: var(--magenta-soft);
    color: var(--magenta);
  }
</style>
