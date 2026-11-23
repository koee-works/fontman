<script lang="ts">
  // The number beside a slider, which you can also type into. At rest it
  // reads as plain text, so a row of sliders stays quiet; hover shows that it
  // can be edited, and a click selects the number for typing.
  //
  // Enter or leaving the field applies the value, clamped to the range.
  // Escape puts back the old one, and so does anything that isn't a number.
  // Up and Down step by `step`, ten steps with Shift. A unit typed after the
  // number ("48px") is ignored.
  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    /** Shown after the number when not editing: "px", "ch". */
    unit?: string;
    /** Decimal places shown. Typed values keep up to this many. */
    decimals?: number;
    /** What the number is: "Preview size", "Weight". */
    label: string;
    disabled?: boolean;
  }
  let {
    value = $bindable(),
    min,
    max,
    step = 1,
    unit = '',
    decimals = 0,
    label,
    disabled = false,
  }: Props = $props();

  let editing = $state(false);
  let draft = $state('');
  /** The mouseup after a click-to-focus would undo the selection. */
  let keepSelection = false;
  let input = $state<HTMLInputElement>();

  // The value can be briefly unset while its owner fills it in.
  const shown = $derived(Number.isFinite(value) ? value.toFixed(decimals) : '');
  const round = (n: number) => Number(n.toFixed(decimals));
  const clamp = (n: number) => Math.min(max, Math.max(min, n));

  function begin() {
    if (disabled) return;
    draft = shown;
    editing = true;
    keepSelection = true;
    // Selected, so typing replaces it.
    queueMicrotask(() => input?.select());
  }

  function commit() {
    if (!editing) return;
    editing = false;
    const n = parseFloat(draft.replace(',', '.'));
    if (Number.isFinite(n)) value = clamp(round(n));
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
      input?.blur();
    } else if (e.key === 'Escape') {
      // Don't let the Escape close a dialog or clear a selection too.
      e.preventDefault();
      e.stopPropagation();
      editing = false;
      input?.blur();
    } else if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      const base = parseFloat(draft);
      const by = (e.key === 'ArrowUp' ? 1 : -1) * step * (e.shiftKey ? 10 : 1);
      value = clamp(round((Number.isFinite(base) ? base : Number.isFinite(value) ? value : min) + by));
      draft = value.toFixed(decimals);
      queueMicrotask(() => input?.select());
    }
  }
</script>

<span class="fm-number" class:editing class:disabled>
  <input
    bind:this={input}
    value={editing ? draft : shown}
    oninput={(e) => (draft = e.currentTarget.value)}
    onfocus={begin}
    onmouseup={(e) => {
      if (keepSelection) e.preventDefault();
      keepSelection = false;
    }}
    onblur={commit}
    onkeydown={onKeydown}
    {disabled}
    inputmode="decimal"
    spellcheck="false"
    autocomplete="off"
    aria-label={label}
    role="spinbutton"
    aria-valuenow={value}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuetext={`${shown}${unit}`}
    size={Math.max(2, (editing ? draft : shown).length)}
  />{#if unit && !editing}<span class="unit" aria-hidden="true">{unit}</span>{/if}
</span>

<style>
  .fm-number {
    display: inline-flex;
    align-items: baseline;
    justify-content: flex-end;
    min-width: 44px;
    height: 22px;
    padding: 0 var(--s-1);
    margin-right: calc(var(--s-1) * -1);
    border: 1px solid transparent;
    border-radius: var(--r-1);
    color: inherit;
    font-size: var(--text-1);
    font-variant-numeric: tabular-nums;
    line-height: 20px;
    cursor: text;
    transition:
      background-color var(--dur-fast) var(--ease),
      border-color var(--dur-fast) var(--ease);
  }
  .fm-number:not(.disabled):hover {
    background: var(--hover);
  }
  .fm-number.editing {
    background: var(--sheet);
    border-color: var(--focus);
    color: var(--ink);
  }
  .fm-number.disabled {
    opacity: 0.5;
    cursor: default;
  }
  input {
    width: auto;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: right;
    outline: none;
    cursor: inherit;
  }
  .unit {
    color: inherit;
  }
</style>
