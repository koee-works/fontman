<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FamilySummary, FaceSummary } from './api';
  import { specimen } from './fonts';
  import { draggableFamily, type DragPayload } from './drag';
  import ActiveToggle from './ui/ActiveToggle.svelte';
  import Checkbox from './ui/Checkbox.svelte';
  import Icon from './ui/Icon.svelte';

  interface Props {
    family: FamilySummary;
    sample: string;
    size: number;
    expanded: boolean;
    selectedFaceId: string | null;
    conflictIds: Set<string>;
    busy: boolean;
    /** Switch a family's library styles on or off. */
    onToggleFamily: (family: FamilySummary, active: boolean) => void;
    onToggleFace: (face: FaceSummary, active: boolean) => void;
    /** Selected style ids, across the whole library. */
    selectedFaces: Set<string>;
    onSelectFaces: (ids: string[], selected: boolean) => void;
    onSelect: (faceId: string) => void;
    onExpand: (name: string) => void;
    /** Extra controls at the end of the header, revealed on hover or focus. */
    actions?: Snippet<[FamilySummary]>;
    /** What dragging this row carries: the family, or the whole selection. */
    dragPayload: (family: FamilySummary) => DragPayload;
    /** Names of the family's tags. */
    tags?: string[];
  }

  let {
    family,
    sample,
    size,
    expanded,
    selectedFaceId,
    conflictIds,
    busy,
    onToggleFamily,
    onToggleFace,
    selectedFaces,
    onSelectFaces,
    onSelect,
    onExpand,
    actions,
    dragPayload,
    tags = [],
  }: Props = $props();

  // An empty sample shows each family's name in its own face.
  const text = $derived(sample.trim() || family.name);
  const faceSize = $derived(Math.min(size, 28));
  const styles = $derived(family.faces.length === 1 ? '1 style' : `${family.faces.length} styles`);
  const hasConflict = $derived(family.faces.some((f) => conflictIds.has(f.id)));
  const toggleable = $derived(family.faces.filter((f) => !f.installed));
  const onCount = $derived(toggleable.filter((f) => f.active).length);
  const picked = $derived(family.faces.filter((f) => selectedFaces.has(f.id)).length);
  const repVariation = $derived(
    family.faces.find((f) => f.id === family.representative)?.variation ?? 'normal',
  );
  const openHere = $derived(family.faces.some((f) => f.id === selectedFaceId));
</script>

<article
  class="family"
  data-family={family.name}
  class:current={openHere}
  aria-label={family.name}
  use:draggableFamily={() => dragPayload(family)}
>
  <header>
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
      onToggle={(on) => onToggleFamily(family, on)}
    />

    <button class="name" onclick={() => onExpand(family.name)} aria-expanded={expanded}>
      <Icon name={expanded ? 'chevron-down' : 'chevron-right'} size={14} />
      <span>{family.name}</span>
    </button>

    <span class="meta">
      {styles}{#if family.variable}, variable{/if}{#if family.installed}, installed{/if}
    </span>
    {#each tags.slice(0, 4) as t (t)}<span class="tag">{t}</span>{/each}
    {#if tags.length > 4}<span class="tag">+{tags.length - 4}</span>{/if}

    {#if hasConflict}
      <span class="badge conflict" title="An active font elsewhere uses the same PostScript name">
        <Icon name="warning" size={12} />
        Name conflict
      </span>
    {/if}
    {#if actions}<span class="actions">{@render actions(family)}</span>{/if}
  </header>

  <button
    class="specimen"
    class:current={selectedFaceId === family.representative}
    style:font-size="{size}px"
    style:font-variation-settings={repVariation}
    use:specimen={family.representative}
    onclick={() => onSelect(family.representative)}
    aria-label="Open {family.name} in the inspector"
  >
    {text}
  </button>

  {#if expanded}
    <ul class="faces">
      {#each family.faces as face (face.id)}
        <li class:current={selectedFaceId === face.id}>
          <span class="check">
            <Checkbox
              checked={selectedFaces.has(face.id)}
              label="Select {family.name} {face.subfamily}"
              onCheckedChange={(on) => onSelectFaces([face.id], on)}
            />
          </span>
          <ActiveToggle
            state={face.installed ? 'installed' : face.active ? 'all' : 'none'}
            name="{family.name} {face.subfamily}"
            disabled={busy}
            onToggle={(on) => onToggleFace(face, on)}
          />
          <button class="face" onclick={() => onSelect(face.id)}>
            <span class="label">
              <span title={face.subfamily}>{face.subfamily}</span>
              {#if conflictIds.has(face.id)}
                <span class="badge conflict"><Icon name="warning" size={12} />Name conflict</span>
              {/if}
            </span>
            <span
              class="face-specimen"
              style:font-size="{faceSize}px"
              style:font-variation-settings={face.variation ?? 'normal'}
              use:specimen={face.id}
            >
              {text}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</article>

<style>
  .family {
    position: relative;
    background: var(--sheet);
    padding: var(--s-3) var(--s-5) var(--s-4);
    border-bottom: 1px solid var(--faint);
  }
  /* Selection is shown around the specimen, never by recolouring it: a
     specimen always renders in true ink, because that's what's being judged. */
  .family.current {
    background: var(--selected);
  }
  .family.current::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: var(--cyan-solid);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 24px;
    color: var(--muted);
  }
  .check {
    display: grid;
    place-items: center;
    width: 18px;
    flex: none;
  }

  .name {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    min-width: 0;
    padding: 2px var(--s-1) 2px 2px;
    margin-left: -2px;
    border-radius: var(--r-1);
    color: var(--ink);
    font-weight: var(--weight-strong);
  }
  .name span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name :global(svg) {
    color: var(--muted);
  }
  .name:hover {
    background: var(--hover);
  }
  .family.current .name:hover {
    background: color-mix(in srgb, var(--sheet) 60%, transparent);
  }
  .meta {
    font-size: var(--text-1);
    white-space: nowrap;
  }

  .tag {
    height: 18px;
    padding: 0 var(--s-2);
    border-radius: var(--r-full);
    background: var(--hover);
    font-size: var(--text-1);
    line-height: 18px;
    white-space: nowrap;
    max-width: 16ch;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .family.current .tag {
    background: color-mix(in srgb, var(--sheet) 60%, transparent);
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: 20px;
    padding: 0 var(--s-2);
    border-radius: var(--r-full);
    font-size: var(--text-1);
    font-weight: 500;
    white-space: nowrap;
  }
  .badge.conflict {
    background: var(--magenta-soft);
    color: var(--magenta);
  }
  header .badge,
  .actions {
    margin-left: auto;
  }
  header .badge + .actions {
    margin-left: 0;
  }
  /* Revealed on hover or keyboard focus, kept while its menu is open, and
     always shown on touch screens. */
  .actions {
    display: flex;
    margin-block: -4px;
    opacity: 0;
  }
  .family:hover .actions,
  .family:focus-within .actions,
  .actions:has(:global([data-state='open'])),
  .actions:has(:global([data-member])) {
    opacity: 1;
  }
  @media (hover: none) {
    .actions {
      opacity: 1;
    }
  }

  .specimen {
    display: block;
    width: 100%;
    text-align: left;
    margin-top: var(--s-1);
    /* Under the family name: checkbox, toggle and their gaps. */
    padding: 2px 0 2px 56px;
    line-height: 1.2;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ink);
    border-radius: var(--r-1);
    /* Specimens render at whatever weight the face is; never synthesize. */
    font-synthesis: none;
  }

  /* Each face row is one surface: it carries hover and selection, with its
     content inset so the selection border never touches the checkbox. The
     negative margin cancels that inset, so face checkboxes line up under the
     family checkbox and face names under the family name. */
  /* Styles sit one column in from their family: each style's checkbox lines
     up under the family's on/off circle. */
  .faces {
    list-style: none;
    margin: var(--s-2) calc(var(--s-2) * -1) 0 calc(30px - var(--s-2));
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .faces li {
    display: grid;
    grid-template-columns: 18px 22px 1fr;
    align-items: center;
    gap: var(--s-2);
    padding: 2px var(--s-2);
    border: 1px solid transparent;
    border-radius: var(--r-2);
    transition: background-color var(--dur-fast) var(--ease);
  }
  .faces li:hover {
    background: var(--hover);
  }
  .family.current .faces li:hover {
    background: color-mix(in srgb, var(--sheet) 60%, transparent);
  }
  .faces li.current {
    background: var(--sheet);
    border-color: var(--cyan-solid);
  }
  .face {
    display: grid;
    grid-template-columns: 96px 1fr;
    align-items: center;
    gap: var(--s-3);
    min-width: 0;
    padding: 2px 0;
    text-align: left;
    border-radius: var(--r-1);
  }
  .label {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-width: 0;
    color: var(--muted);
    font-size: var(--text-1);
  }
  .label > span:first-child {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .face-specimen {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.25;
    color: var(--ink);
    font-synthesis: none;
  }
</style>
