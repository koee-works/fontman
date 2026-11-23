<script lang="ts">
  import { Popover } from 'bits-ui';
  import type { Tag } from '../api';
  import Icon from './Icon.svelte';

  // A family's tags as removable chips, plus a popover to add one: type to
  // filter existing tags, or to name a new one.
  interface Props {
    /** The family's display name. */
    family: string;
    tags: Tag[];
    onAdd: (tag: Tag) => void;
    onRemove: (tag: Tag) => void;
    /** Create a tag on this family. Resolves to an error message, or null. */
    onCreate: (name: string) => Promise<string | null>;
  }
  let { family, tags, onAdd, onRemove, onCreate }: Props = $props();

  let open = $state(false);
  let text = $state('');
  let error = $state<string | null>(null);
  let active = $state(0);

  const key = $derived(family.toLowerCase());
  const on = $derived(tags.filter((t) => t.families.includes(key)));
  const matches = $derived.by(() => {
    const q = text.trim().toLowerCase();
    return tags.filter((t) => !t.families.includes(key) && (!q || t.name.toLowerCase().includes(q)));
  });
  /** Offer to create a tag when the text names none that exists. */
  const canCreate = $derived(
    text.trim().length > 0 && !tags.some((t) => t.name.toLowerCase() === text.trim().toLowerCase()),
  );
  const options = $derived(matches.length + (canCreate ? 1 : 0));

  $effect(() => {
    if (open) {
      text = '';
      error = null;
      active = 0;
    }
  });
  $effect(() => {
    // Keep the highlight on a real option as the list changes.
    if (active >= options) active = Math.max(0, options - 1);
  });

  async function choose(i: number) {
    if (i < matches.length) {
      onAdd(matches[i]!);
      open = false;
    } else if (canCreate) {
      error = await onCreate(text.trim());
      if (!error) open = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(options - 1, active + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(0, active - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (options) choose(active);
    }
  }
</script>

<div class="tags">
  {#each on as t (t.id)}
    <span class="chip">
      {t.name}
      <button class="remove" aria-label="Remove the tag {t.name} from {family}" onclick={() => onRemove(t)}>
        <Icon name="x" size={12} />
      </button>
    </span>
  {/each}
  <Popover.Root bind:open>
    <Popover.Trigger class="fm-add-tag" aria-label="Add a tag to {family}">
      <Icon name="tag-plus" size={12} />
      {on.length ? 'Tag' : 'Add a tag'}
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content class="fm-menu fm-tag-pop" sideOffset={4} align="start">
        <input
          class="tag-input"
          bind:value={text}
          placeholder="Find or create a tag"
          aria-label="Tag name"
          maxlength="80"
          spellcheck="false"
          autocomplete="off"
          role="combobox"
          aria-expanded="true"
          aria-controls="tag-options"
          aria-activedescendant={options ? `tag-option-${active}` : undefined}
          onkeydown={onKeydown}
          oninput={() => (error = null)}
        />
        <ul class="options" id="tag-options" role="listbox" aria-label="Tags">
          <!-- Combobox pattern: the keyboard stays in the input, which moves the
               highlight (aria-activedescendant); options take pointer clicks only. -->
          {#each matches as t, i (t.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
              id="tag-option-{i}"
              role="option"
              aria-selected={i === active}
              class="fm-menu-item"
              data-highlighted={i === active ? '' : undefined}
              onpointerenter={() => (active = i)}
              onclick={() => choose(i)}
            >
              <span class="fm-menu-check"><Icon name="tag" size={14} /></span>
              <span class="name">{t.name}</span>
              {#if t.source === 'google'}<span class="grp">{t.group || 'Google'}</span>{/if}
            </li>
          {/each}
          {#if canCreate}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li
              id="tag-option-{matches.length}"
              role="option"
              aria-selected={active === matches.length}
              class="fm-menu-item"
              data-highlighted={active === matches.length ? '' : undefined}
              onpointerenter={() => (active = matches.length)}
              onclick={() => choose(matches.length)}
            >
              <span class="fm-menu-check"><Icon name="tag-plus" size={14} /></span>
              <span class="name">Create “{text.trim()}”</span>
            </li>
          {/if}
          {#if !options}
            <li class="none">{tags.length ? 'Type a name for a new tag.' : 'No tags yet. Type a name to create one.'}</li>
          {/if}
        </ul>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s-1);
    min-width: 0;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 22px;
    padding: 0 2px 0 var(--s-2);
    border-radius: var(--r-full);
    background: var(--hover);
    color: var(--ink);
    font-size: var(--text-1);
  }
  .remove {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: var(--r-full);
    color: var(--muted);
  }
  .remove:hover {
    background: var(--pressed);
    color: var(--ink);
  }
  :global(.fm-add-tag) {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 22px;
    padding: 0 var(--s-2);
    border: 1px dashed var(--line);
    border-radius: var(--r-full);
    color: var(--muted);
    font-size: var(--text-1);
  }
  :global(.fm-add-tag:hover),
  :global(.fm-add-tag[data-state='open']) {
    color: var(--ink);
    border-color: var(--muted);
  }
  :global(.fm-tag-pop) {
    width: 240px;
  }
  .tag-input {
    width: 100%;
    height: var(--control);
    margin-bottom: var(--s-1);
    padding: 0 var(--s-2);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    color: var(--ink);
  }
  .tag-input:focus-visible {
    outline-offset: 0;
    border-color: var(--focus);
  }
  .options {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 220px;
    overflow-y: auto;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grp {
    margin-left: auto;
    padding-left: var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
    white-space: nowrap;
  }
  .none {
    padding: var(--s-2);
    color: var(--muted);
    font-size: var(--text-1);
  }
  .error {
    margin: var(--s-1) var(--s-2);
    color: var(--red);
    font-size: var(--text-1);
  }
</style>
