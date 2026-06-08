<script lang="ts">
  import { startDrag } from '../../lib/browser/useDrag.js';

  let {
    onmove,
    onend = undefined,
    style = '',
    label = 'Drag handle',
    variant = 'corner',
  }: {
    onmove: (dx: number, dy: number) => void;
    onend?: () => void;
    style?: string;
    label?: string;
    variant?: 'corner' | 'edge' | 'body';
  } = $props();

  function down(e: PointerEvent) {
    startDrag(e, { onMove: (dx, dy) => onmove(dx, dy), onEnd: () => onend?.() });
  }
</script>

<button type="button" class="ed-handle ed-{variant}" {style} aria-label={label} onpointerdown={down}></button>

<style>
  .ed-handle {
    position: absolute;
    padding: 0;
    background: var(--accent-500);
    border: 2px solid #fff;
    border-radius: 50%;
    box-shadow: var(--shadow-md);
    cursor: grab;
    touch-action: none;
    z-index: 3;
  }
  .ed-corner,
  .ed-edge { width: 15px; height: 15px; }
  /* generous invisible hit area for touch (>= 44px) */
  .ed-handle::before { content: ''; position: absolute; inset: -16px; }
  .ed-handle:active { cursor: grabbing; }
  @media (prefers-reduced-motion: no-preference) {
    .ed-handle { transition: transform var(--dur-fast) var(--ease-out); }
    .ed-handle:active { transform: scale(1.25); }
  }
</style>
