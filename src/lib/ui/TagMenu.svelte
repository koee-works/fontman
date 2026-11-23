<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import type { Tag } from '../api';
  import Icon from './Icon.svelte';

  // Tag some families, or untag them. A tick means every one of them has the
  // tag; a dash means some do.
  interface Props {
    tags: Tag[];
    /** Family display names. */
    families: string[];
    label: string;
    text: string;
    onToggle: (tag: Tag, tagged: boolean) => void;
    onCreate: () => void;
  }
  let { tags, families, label, text, onToggle, onCreate }: Props = $props();

  const keys = $derived(families.map((f) => f.toLowerCase()));
  function state(t: Tag): 'all' | 'some' | 'none' {
    const n = keys.filter((k) => t.families.includes(k)).length;
    return n === 0 ? 'none' : n === keys.length ? 'all' : 'some';
  }
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger class="fm-menu-btn" aria-label={label}>
    <Icon name="tag" size={14} />
    {text}
    <Icon name="chevron-down" size={12} />
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content class="fm-menu collection-menu" sideOffset={4} align="end">
      {#if tags.length}
        <DropdownMenu.Group>
          <DropdownMenu.GroupHeading class="fm-menu-heading">Tags</DropdownMenu.GroupHeading>
          {#each tags as t (t.id)}
            {@const st = state(t)}
            <DropdownMenu.CheckboxItem
              class="fm-menu-item"
              checked={st === 'all'}
              indeterminate={st === 'some'}
              onCheckedChange={() => onToggle(t, st !== 'all')}
            >
              <span class="fm-menu-check">
                {#if st === 'all'}<Icon name="check" size={14} />{:else if st === 'some'}<Icon name="minus" size={14} />{/if}
              </span>
              <span class="name">{t.name}</span>
            </DropdownMenu.CheckboxItem>
          {/each}
        </DropdownMenu.Group>
        <DropdownMenu.Separator class="fm-menu-separator" />
      {/if}
      <DropdownMenu.Item class="fm-menu-item" onSelect={onCreate}>
        <span class="fm-menu-check"><Icon name="tag-plus" size={14} /></span>
        New tag…
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
