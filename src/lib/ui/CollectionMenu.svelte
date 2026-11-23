<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import type { Collection } from '../api';
  import Icon from './Icon.svelte';

  // Add some faces to collections, or take them out. A tick means every one
  // of the faces is in that collection; a dash means some are.
  interface Props {
    collections: Collection[];
    /** The faces this menu acts on: one style, or every style of a family. */
    faceIds: string[];
    /** Accessible name of the trigger, e.g. "Add Inter to a collection". */
    label: string;
    /** Visible trigger text. Without it the trigger is an icon button. */
    text?: string;
    onToggle: (collection: Collection, member: boolean) => void;
    onCreate: () => void;
  }
  let { collections, faceIds, label, text, onToggle, onCreate }: Props = $props();

  function state(c: Collection): 'all' | 'some' | 'none' {
    const set = new Set(c.faceIds);
    const n = faceIds.filter((id) => set.has(id)).length;
    return n === 0 ? 'none' : n === faceIds.length ? 'all' : 'some';
  }
  const inAny = $derived(collections.some((c) => state(c) !== 'none'));
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    class={text ? 'fm-menu-btn' : 'fm-icon-btn sm'}
    aria-label={label}
    data-member={inAny ? '' : undefined}
  >
    <Icon name={inAny ? 'bookmark' : 'bookmark-plus'} size={14} />
    {#if text}{text}{/if}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content class="fm-menu collection-menu" sideOffset={4} align="end">
      {#if collections.length}
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading class="fm-menu-heading">Collections</DropdownMenu.GroupHeading>
          {#each collections as c (c.id)}
            {@const st = state(c)}
            <DropdownMenu.CheckboxItem
              class="fm-menu-item"
              checked={st === 'all'}
              indeterminate={st === 'some'}
              onCheckedChange={() => onToggle(c, st !== 'all')}
            >
              <span class="fm-menu-check">
                {#if st === 'all'}<Icon name="check" size={14} />{:else if st === 'some'}<Icon
                    name="minus"
                    size={14}
                  />{/if}
              </span>
              <span class="name">{c.name}</span>
            </DropdownMenu.CheckboxItem>
          {/each}
        </DropdownMenu.Group>
        <DropdownMenu.Separator class="fm-menu-separator" />
      {/if}
      <DropdownMenu.Item class="fm-menu-item" onSelect={onCreate}>
        <span class="fm-menu-check"><Icon name="bookmark-plus" size={14} /></span>
        New collection…
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  :global(.collection-menu) {
    max-width: 280px;
    max-height: min(420px, var(--bits-dropdown-menu-content-available-height, 420px));
    overflow-y: auto;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
