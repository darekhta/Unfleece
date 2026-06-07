// Minimal pointer-drag helper — unified mouse/touch/pen via Pointer Events, with
// pointer capture so a drag keeps tracking even if it leaves the element. Emits
// incremental deltas (CSS px) since the last move; the consumer accumulates +
// clamps. Pair with `touch-action: none` on the draggable to stop scroll-hijack.

export interface DragHandlers {
  onMove: (dx: number, dy: number, e: PointerEvent) => void;
  onStart?: () => void;
  onEnd?: () => void;
}

export function startDrag(e: PointerEvent, h: DragHandlers): void {
  e.preventDefault();
  e.stopPropagation();
  const target = e.currentTarget as HTMLElement;
  const id = e.pointerId;
  let lastX = e.clientX;
  let lastY = e.clientY;
  try {
    target.setPointerCapture(id);
  } catch {
    /* noop */
  }
  h.onStart?.();

  function move(ev: PointerEvent) {
    if (ev.pointerId !== id) return;
    const dx = ev.clientX - lastX;
    const dy = ev.clientY - lastY;
    lastX = ev.clientX;
    lastY = ev.clientY;
    h.onMove(dx, dy, ev);
  }
  function end(ev: PointerEvent) {
    if (ev.pointerId !== id) return;
    target.removeEventListener('pointermove', move);
    target.removeEventListener('pointerup', end);
    target.removeEventListener('pointercancel', end);
    try {
      target.releasePointerCapture(id);
    } catch {
      /* noop */
    }
    h.onEnd?.();
  }
  target.addEventListener('pointermove', move);
  target.addEventListener('pointerup', end);
  target.addEventListener('pointercancel', end);
}
