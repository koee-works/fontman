<script lang="ts">
  import { Slider } from 'bits-ui';

  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    label: string;
    disabled?: boolean;
  }
  let { value = $bindable(), min, max, step = 1, label, disabled = false }: Props = $props();
</script>

<Slider.Root type="single" bind:value {min} {max} {step} {disabled} class="fm-slider">
  {#snippet children({ thumbs })}
    <span class="fm-slider-track">
      <Slider.Range class="fm-slider-range" />
    </span>
    {#each thumbs as index (index)}
      <Slider.Thumb {index} class="fm-slider-thumb" aria-label={label} />
    {/each}
  {/snippet}
</Slider.Root>

<style>
  :global(.fm-slider) {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: 20px;
    touch-action: none;
    user-select: none;
  }
  :global(.fm-slider[data-disabled]) {
    opacity: 0.5;
  }
  :global(.fm-slider-track) {
    position: relative;
    flex: 1;
    height: 4px;
    border-radius: var(--r-full);
    background: var(--faint);
    overflow: hidden;
  }
  :global(.fm-slider-range) {
    position: absolute;
    height: 100%;
    background: var(--cyan-solid);
  }
  :global(.fm-slider-thumb) {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: var(--r-full);
    background: var(--sheet);
    border: 1.5px solid var(--cyan-solid);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.15);
    cursor: grab;
    transition: transform var(--dur-fast) var(--ease);
  }
  :global(.fm-slider-thumb:active) {
    cursor: grabbing;
    transform: scale(1.1);
  }
  :global(.fm-slider-thumb:focus-visible) {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
</style>
