<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ContextMenu } from 'bits-ui';
  import type { Collection, FamilySummary, Tag } from './api';
  import Icon from './ui/Icon.svelte';

  // The right-click menu for a family, wherever families are listed. One menu
  // serves every row and card: rows are virtualised, so rather than a menu
  // per row, the menu finds the family under the pointer by `data-family`.
  // Right-clicking anywhere else in the area shows nothing, not even the
  // browser's own menu. The menu key and Shift+F10 open it for the focused
  // row, placed under the focused control.
  interface Props {
    children: Snippet;
    familyOf: (name: string) => FamilySummary | undefined;
    busy: boolean;
    /** Style rows can be shown, in the list layout. */
    canExpand: boolean;
    isExpanded: (name: string) => boolean;
    onExpand: (name: string) => void;
    selectedFaces: Set<string>;
    onSelectFaces: (ids: string[], on: boolean) => void;
    onOpen: (faceId: string) => void;
    onToggleFamily: (family: FamilySummary, active: boolean) => void;
    collections: Collection[];
    onCollection: (c: Collection, faceIds: string[], member: boolean, what: string) => void;
    onNewCollection: (faceIds: string[], what: string) => void;
    tags: Tag[];
    onTag: (t: Tag, families: string[], tagged: boolean, what: string) => void;
    onNewTag: (families: string[], what: string) => void;
    onCompare: (family: FamilySummary) => void;
    onViewer: (role: 'heading' | 'body', family: FamilySummary) => void;
    onCopyName: (family: FamilySummary) => void;
    onReveal: (family: FamilySummary) => void;
    /** The collection being viewed, which the family can be taken out of. */
    collection?: Collection;
  }
  let {
    children,
    familyOf,
    busy,
    canExpand,
    isExpanded,
    onExpand,
    selectedFaces,
    onSelectFaces,
    onOpen,
    onToggleFamily,
    collections,
    onCollection,
    onNewCollection,
    tags,
    onTag,
    onNewTag,
    onCompare,
    onViewer,
    onCopyName,
    onReveal,
    collection,
  }: Props = $props();

  let open = $state(false);
  let name = $state<string | null>(null);
  /** The row or card the menu is for, marked while the menu is open. */
  let marked: HTMLElement | null = null;
  /** Where focus goes back to after a menu opened from the keyboard. */
  let returnTo: HTMLElement | null = null;
  let fromKeyboard = false;
  let suppressUntil = 0;

  // Resolved live, so the menu reflects changes while it's open.
  const family = $derived(name ? familyOf(name) : undefined);
  const ids = $derived(family?.faces.map((f) => f.id) ?? []);
  const lib = $derived(family?.faces.filter((f) => !f.installed) ?? []);
  const onCount = $derived(lib.filter((f) => f.active).length);
  const picked = $derived(ids.filter((id) => selectedFaces.has(id)).length);
  const memberOfCurrent = $derived(
    !!collection && ids.some((id) => collection.faceIds.includes(id)),
  );

  function mark(el: HTMLElement | null) {
    marked?.removeAttribute('data-menu-target');
    marked = el;
    marked?.setAttribute('data-menu-target', '');
  }

  function onContextMenu(e: MouseEvent) {
    // The native menu the key would also have raised, after ours.
    if (e.isTrusted && performance.now() < suppressUntil) {
      e.preventDefault();
      return;
    }
    const el = (e.target as Element | null)?.closest<HTMLElement>('[data-family]');
    const fam = el?.dataset.family;
    if (!el || !fam || !familyOf(fam)) {
      // Not on a family: no menu at all. Default-prevented, Bits ignores it.
      e.preventDefault();
      return;
    }
    name = fam;
    mark(el);
    returnTo = fromKeyboard ? (document.activeElement as HTMLElement | null) : null;
    fromKeyboard = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!(e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey))) return;
    const focused = document.activeElement as HTMLElement | null;
    if (!focused?.closest('[data-family]')) return;
    e.preventDefault();
    const r = focused.getBoundingClientRect();
    fromKeyboard = true;
    suppressUntil = performance.now() + 250;
    focused.dispatchEvent(
      new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        clientX: Math.round(r.left + Math.min(24, r.width / 2)),
        clientY: Math.round(r.bottom),
      }),
    );
  }

  function onOpenChange(v: boolean) {
    if (!v) mark(null);
  }

  /** Run an action once the menu has closed, so dialogs can take focus. */
  const later = (fn: () => void) => () => setTimeout(fn, 0);
</script>

<ContextMenu.Root bind:open {onOpenChange}>
  <ContextMenu.Trigger class="fm-context-area" oncontextmenu={onContextMenu} onkeydown={onKeydown}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content
      class="fm-menu"
      onCloseAutoFocus={(e) => {
        e.preventDefault();
        const el = returnTo;
        returnTo = null;
        // Back to the row, unless the action moved focus somewhere on purpose.
        setTimeout(() => {
          if (el?.isConnected && (!document.activeElement || document.activeElement === document.body)) el.focus();
        }, 0);
      }}
    >
      {#if family}
        <ContextMenu.Item class="fm-menu-item" onSelect={() => onOpen(family.representative)}>
          <span class="fm-menu-check"></span>
          Open in inspector
        </ContextMenu.Item>
        {#if canExpand}
          <ContextMenu.Item class="fm-menu-item" onSelect={() => onExpand(family.name)}>
            <span class="fm-menu-check"></span>
            {isExpanded(family.name) ? 'Hide styles' : 'Show styles'}
          </ContextMenu.Item>
        {/if}
        <ContextMenu.Item class="fm-menu-item" onSelect={() => onSelectFaces(ids, picked < ids.length)}>
          <span class="fm-menu-check"></span>
          {picked === ids.length ? 'Deselect family' : 'Select family'}
        </ContextMenu.Item>

        <ContextMenu.Separator class="fm-menu-separator" />
        {#if !lib.length}
          <ContextMenu.Item class="fm-menu-item" disabled>
            <span class="fm-menu-check"><Icon name="lock" size={14} /></span>
            Installed with the system
          </ContextMenu.Item>
        {:else}
          {#if onCount < lib.length}
            <ContextMenu.Item class="fm-menu-item" disabled={busy} onSelect={() => onToggleFamily(family, true)}>
              <span class="fm-menu-check"></span>
              {onCount ? `Activate all ${lib.length} styles` : 'Activate family'}
            </ContextMenu.Item>
          {/if}
          {#if onCount > 0}
            <ContextMenu.Item class="fm-menu-item" disabled={busy} onSelect={() => onToggleFamily(family, false)}>
              <span class="fm-menu-check"></span>
              Deactivate family
            </ContextMenu.Item>
          {/if}
        {/if}

        <ContextMenu.Separator class="fm-menu-separator" />
        <ContextMenu.Sub>
          <ContextMenu.SubTrigger class="fm-menu-item">
            <span class="fm-menu-check"><Icon name="bookmark" size={14} /></span>
            <span class="grow">Collections</span>
            <Icon name="chevron-right" size={12} />
          </ContextMenu.SubTrigger>
          <ContextMenu.SubContent class="fm-menu collection-menu" sideOffset={4} align="start" alignOffset={-4}>
            {#each collections as c (c.id)}
              {@const n = ids.filter((id) => c.faceIds.includes(id)).length}
              {@const st = n === 0 ? 'none' : n === ids.length ? 'all' : 'some'}
              <ContextMenu.CheckboxItem
                class="fm-menu-item"
                checked={st === 'all'}
                indeterminate={st === 'some'}
                closeOnSelect={false}
                onCheckedChange={() => onCollection(c, ids, st !== 'all', family.name)}
              >
                <span class="fm-menu-check">
                  {#if st === 'all'}<Icon name="check" size={14} />{:else if st === 'some'}<Icon name="minus" size={14} />{/if}
                </span>
                <span class="grow">{c.name}</span>
              </ContextMenu.CheckboxItem>
            {/each}
            {#if collections.length}<ContextMenu.Separator class="fm-menu-separator" />{/if}
            <ContextMenu.Item class="fm-menu-item" onSelect={later(() => onNewCollection(ids, family.name))}>
              <span class="fm-menu-check"><Icon name="bookmark-plus" size={14} /></span>
              New collection…
            </ContextMenu.Item>
          </ContextMenu.SubContent>
        </ContextMenu.Sub>
        <ContextMenu.Sub>
          <ContextMenu.SubTrigger class="fm-menu-item">
            <span class="fm-menu-check"><Icon name="tag" size={14} /></span>
            <span class="grow">Tags</span>
            <Icon name="chevron-right" size={12} />
          </ContextMenu.SubTrigger>
          <ContextMenu.SubContent class="fm-menu collection-menu" sideOffset={4} align="start" alignOffset={-4}>
            {#each tags as t (t.id)}
              {@const on = t.families.includes(family.name.toLowerCase())}
              <ContextMenu.CheckboxItem
                class="fm-menu-item"
                checked={on}
                closeOnSelect={false}
                onCheckedChange={() => onTag(t, [family.name], !on, family.name)}
              >
                <span class="fm-menu-check">{#if on}<Icon name="check" size={14} />{/if}</span>
                <span class="grow">{t.name}</span>
              </ContextMenu.CheckboxItem>
            {/each}
            {#if tags.length}<ContextMenu.Separator class="fm-menu-separator" />{/if}
            <ContextMenu.Item class="fm-menu-item" onSelect={later(() => onNewTag([family.name], family.name))}>
              <span class="fm-menu-check"><Icon name="tag-plus" size={14} /></span>
              New tag…
            </ContextMenu.Item>
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Separator class="fm-menu-separator" />
        <ContextMenu.Item class="fm-menu-item" onSelect={() => onCompare(family)}>
          <span class="fm-menu-check"><Icon name="compare" size={14} /></span>
          Compare with another style
        </ContextMenu.Item>
        <ContextMenu.Sub>
          <ContextMenu.SubTrigger class="fm-menu-item">
            <span class="fm-menu-check"><Icon name="style-viewer" size={14} /></span>
            <span class="grow">Try in style viewer</span>
            <Icon name="chevron-right" size={12} />
          </ContextMenu.SubTrigger>
          <ContextMenu.SubContent class="fm-menu" sideOffset={4} align="start" alignOffset={-4}>
            <ContextMenu.Item class="fm-menu-item" onSelect={() => onViewer('heading', family)}>
              <span class="fm-menu-check"></span>
              As headings
            </ContextMenu.Item>
            <ContextMenu.Item class="fm-menu-item" onSelect={() => onViewer('body', family)}>
              <span class="fm-menu-check"></span>
              As body text
            </ContextMenu.Item>
          </ContextMenu.SubContent>
        </ContextMenu.Sub>

        <ContextMenu.Separator class="fm-menu-separator" />
        <ContextMenu.Item class="fm-menu-item" onSelect={() => onCopyName(family)}>
          <span class="fm-menu-check"><Icon name="copy" size={14} /></span>
          Copy family name
        </ContextMenu.Item>
        <ContextMenu.Item class="fm-menu-item" onSelect={() => onReveal(family)}>
          <span class="fm-menu-check"><Icon name="reveal" size={14} /></span>
          Show in folder
        </ContextMenu.Item>

        {#if collection && memberOfCurrent}
          <ContextMenu.Separator class="fm-menu-separator" />
          <ContextMenu.Item
            class="fm-menu-item"
            onSelect={() => onCollection(collection, ids, false, family.name)}
          >
            <span class="fm-menu-check"></span>
            Remove from {collection.name}
          </ContextMenu.Item>
        {/if}
      {/if}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>

<style>
  /* The trigger wraps the results without adding a box of its own. */
  :global(.fm-context-area) {
    display: contents;
  }
  .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.fm-menu-item[data-state='open']) {
    background: var(--hover);
  }
  :global(.fm-menu-item > svg:last-child) {
    color: var(--muted);
    margin-right: calc(var(--s-1) * -1);
  }
  /* The row or card a context menu is open for. */
  :global([data-family][data-menu-target]) {
    box-shadow: inset 0 0 0 1px var(--focus);
  }
</style>
