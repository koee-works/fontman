<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    value: string;
    placeholder: string;
    label: string;
    size?: 'md' | 'lg';
    /** Exposed so a keyboard shortcut can focus the field. */
    ref?: HTMLInputElement;
  }
  let { value = $bindable(), placeholder, label, size = 'md', ref = $bindable() }: Props = $props();
</script>

<label class="search {size}">
  <span class="visually-hidden">{label}</span>
  <Icon name="search" size={size === 'lg' ? 16 : 14} />
  <input type="search" bind:this={ref} bind:value {placeholder} spellcheck="false" />
  {#if value}
    <button type="button" class="clear" onclick={() => { value = ''; ref?.focus(); }} aria-label="Clear search">
      <Icon name="x" size={12} />
    </button>
  {/if}
</label>

<style>
  .search {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: var(--control);
    padding: 0 var(--s-2) 0 var(--s-3);
    border: 1px solid var(--line);
    border-radius: var(--r-2);
    background: var(--sheet);
    color: var(--muted);
    cursor: text;
  }
  .search:focus-within {
    border-color: var(--focus);
    box-shadow: 0 0 0 1px var(--focus);
  }
  .lg {
    border: 0;
    background: none;
    padding: 0;
    height: auto;
    box-shadow: none !important;
  }
  .lg input {
    font-size: var(--text-3);
  }
  input {
    flex: 1;
    border: 0;
    background: none;
    outline: none;
    color: var(--ink);
    padding: var(--s-1) 0;
  }
  input::-webkit-search-cancel-button {
    display: none;
  }
  input::placeholder {
    color: var(--muted);
  }
  .clear {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: var(--r-full);
    color: var(--muted);
  }
  .clear:hover {
    background: var(--hover);
    color: var(--ink);
  }
</style>
