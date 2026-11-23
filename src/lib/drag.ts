// Drag families (or the current selection) onto collections and tags in the
// sidebar.
//
// Pointer events rather than HTML drag and drop: native drag and drop is
// intercepted by the webview on some platforms (Tauri reserves it for file
// drops on Windows), and pointer events behave the same everywhere. A drag
// starts only after the pointer moves a few pixels, so a click still opens
// the inspector, and Escape cancels.
//
// Drop targets are elements with `data-drop="<kind>:<id>"`. The app sets one
// handler for all drops with `onDrop`.
//
// Elements with `data-spring` open when a drag rests on them: a folded
// section, say, so its drop targets can be reached without letting go. After
// SPRING_MS the element is clicked, once per visit.

export interface DragPayload {
  /** What the ghost says: a family's name, or "3 families". */
  label: string;
  /** Family display names, for tags. */
  families: string[];
  /** Style ids, for collections. */
  faceIds: string[];
}

type DropHandler = (target: string, payload: DragPayload) => void;

let handler: DropHandler | null = null;

/** Set what happens when a family is dropped on a target. */
export function onDrop(fn: DropHandler) {
  handler = fn;
}

const THRESHOLD = 6;
const SPRING_MS = 600;

/** Is there a drag in progress? Lets other code stay out of the way. */
export const dragState = { active: false };

/**
 * Make an element draggable as a family. `payload` is read when the drag
 * starts, so it can change with the element.
 */
export function draggableFamily(node: HTMLElement, payload: () => DragPayload) {
  let get = payload;
  let start: { x: number; y: number; id: number } | null = null;
  let ghost: HTMLElement | null = null;
  let over: HTMLElement | null = null;
  let spring: HTMLElement | null = null;
  let springTimer: ReturnType<typeof setTimeout> | undefined;

  function setSpring(el: HTMLElement | null) {
    if (el === spring) return;
    clearTimeout(springTimer);
    spring?.removeAttribute('data-spring-over');
    spring = el;
    if (!el) return;
    el.setAttribute('data-spring-over', '');
    springTimer = setTimeout(() => {
      if (spring === el && ghost) el.click();
    }, SPRING_MS);
  }

  function targetAt(x: number, y: number): HTMLElement | null {
    const el = document.elementFromPoint(x, y);
    return (el?.closest('[data-drop]') as HTMLElement | null) ?? null;
  }

  function setOver(el: HTMLElement | null) {
    if (el === over) return;
    over?.removeAttribute('data-drop-over');
    over = el;
    over?.setAttribute('data-drop-over', '');
    if (ghost) ghost.dataset.over = over ? 'true' : 'false';
  }

  function begin(e: PointerEvent) {
    const p = get();
    ghost = document.createElement('div');
    ghost.className = 'fm-drag-ghost';
    ghost.textContent = p.label;
    ghost.setAttribute('aria-hidden', 'true');
    document.body.append(ghost);
    document.body.classList.add('fm-dragging');
    dragState.active = true;
    move(e);
  }

  function move(e: PointerEvent) {
    if (!ghost) return;
    ghost.style.translate = `${e.clientX + 12}px ${e.clientY + 8}px`;
    setOver(targetAt(e.clientX, e.clientY));
    const under = document.elementFromPoint(e.clientX, e.clientY);
    setSpring((under?.closest('[data-spring]') as HTMLElement | null) ?? null);
  }

  function end(drop: boolean) {
    const target = over?.dataset.drop;
    setOver(null);
    setSpring(null);
    ghost?.remove();
    const dragged = ghost !== null;
    ghost = null;
    start = null;
    document.body.classList.remove('fm-dragging');
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
    window.removeEventListener('pointercancel', onCancel);
    window.removeEventListener('keydown', onKey, true);
    if (dragged) {
      // The pointerup that ends a drag also fires a click on the element
      // under it; swallow it so a drop doesn't open the inspector too.
      const swallow = (ev: MouseEvent) => {
        ev.stopPropagation();
        ev.preventDefault();
      };
      window.addEventListener('click', swallow, { capture: true, once: true });
      setTimeout(() => window.removeEventListener('click', swallow, { capture: true }), 0);
      // Let the click (if any) pass first, then report the drag over.
      setTimeout(() => (dragState.active = false), 0);
      if (drop && target) handler?.(target, get());
    }
  }

  function onMove(e: PointerEvent) {
    if (!start || e.pointerId !== start.id) return;
    if (!ghost) {
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < THRESHOLD) return;
      begin(e);
    }
    e.preventDefault();
    move(e);
  }
  const onUp = (e: PointerEvent) => start && e.pointerId === start.id && end(true);
  const onCancel = () => end(false);
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && ghost) {
      e.preventDefault();
      e.stopPropagation();
      end(false);
    }
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0 || !e.isPrimary) return;
    // Controls inside the row keep their own pointer behaviour.
    const t = e.target as HTMLElement;
    if (t.closest('[role="checkbox"], input, [data-no-drag], [aria-haspopup]')) return;
    start = { x: e.clientX, y: e.clientY, id: e.pointerId };
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onCancel);
    window.addEventListener('keydown', onKey, true);
  }

  node.addEventListener('pointerdown', onDown);
  return {
    update(next: () => DragPayload) {
      get = next;
    },
    destroy() {
      node.removeEventListener('pointerdown', onDown);
      if (start || ghost) end(false);
    },
  };
}
