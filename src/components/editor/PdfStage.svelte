<script lang="ts">
  import type { FriendlyError } from '../../lib/errors.js';
  import { onMount, onDestroy, type Snippet } from 'svelte';
  import Icon from '../Icon.svelte';
  import { loadPdfDoc, type PdfDoc } from '../../lib/browser/preview.js';
  import type { StageDims } from '../../lib/browser/stageCoords.js';
  import { friendlyError } from '../../lib/errors.js';
  import { focusAfterUpdate } from '../../lib/focus.js';

  export interface StageInfo extends StageDims {
    pageIndex: number;
    numPages: number;
    clientToView: (clientX: number, clientY: number) => { x: number; y: number };
  }

  let {
    bytes,
    pageIndex = $bindable(0),
    maxCssWidth = 820,
    controls = true,
    info = $bindable(),
    overlay,
  }: {
    bytes: Uint8Array;
    pageIndex?: number;
    maxCssWidth?: number;
    controls?: boolean;
    info?: StageInfo;
    overlay?: Snippet<[StageInfo]>;
  } = $props();

  let container: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let overlayEl: HTMLDivElement;
  let errorEl: HTMLDivElement | null = null;

  let doc: PdfDoc | null = null;
  let numPages = $state(0);
  let dims = $state<StageDims>({ scale: 1, cssWidth: 0, cssHeight: 0, pageWidthPt: 0, pageHeightPt: 0 });
  let loading = $state(true);
  let error = $state<FriendlyError | null>(null);
  let renderTask: { cancel: () => void; promise: Promise<void> } | null = null;
  let ro: ResizeObserver | undefined;
  let rafId = 0;

  // Zoom: 1 = fit width. Geometry elsewhere is stored as page fractions, so it
  // survives zoom changes; the canvas just re-renders at the new scale (crisp).
  let zoom = $state(1);
  const ZOOM_MIN = 0.5;
  const ZOOM_MAX = 3;
  function setZoom(z: number) {
    zoom = Math.min(Math.max(Math.round(z * 100) / 100, ZOOM_MIN), ZOOM_MAX);
  }

  async function render() {
    try {
      error = null;
      if (!doc) {
        doc = await loadPdfDoc(bytes);
        numPages = doc.numPages;
      }
      const p = Math.min(Math.max(pageIndex, 0), numPages - 1);
      const page = await doc.getPage(p + 1);
      const base = page.getViewport({ scale: 1 });
      const fitWidth = Math.max(160, Math.min(container?.clientWidth || maxCssWidth, maxCssWidth));
      const cssWidth = Math.round(fitWidth * zoom);
      const scale = cssWidth / base.width;
      const viewport = page.getViewport({ scale });
      const dpr = Math.min(window.devicePixelRatio || 1, 2);

      canvas.width = Math.round(viewport.width * dpr);
      canvas.height = Math.round(viewport.height * dpr);
      canvas.style.width = `${Math.round(viewport.width)}px`;
      canvas.style.height = `${Math.round(viewport.height)}px`;
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas not supported');
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, viewport.width, viewport.height);

      try { renderTask?.cancel(); } catch { /* noop */ }
      renderTask = page.render({ canvasContext: ctx, viewport });
      await renderTask.promise;
      page.cleanup();
      dims = { scale, cssWidth: viewport.width, cssHeight: viewport.height, pageWidthPt: base.width, pageHeightPt: base.height };
      loading = false;
    } catch (e: any) {
      if (e?.name === 'RenderingCancelledException') return;
      error = friendlyError(e, "Couldn't render this PDF");
      loading = false;
      await focusAfterUpdate(() => errorEl);
    }
  }
  function scheduleRender() {
    cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(render);
  }

  onMount(() => {
    render();
    ro = new ResizeObserver(() => scheduleRender());
    if (container) ro.observe(container);
  });
  onDestroy(() => {
    ro?.disconnect();
    cancelAnimationFrame(rafId);
    try { renderTask?.cancel(); } catch { /* noop */ }
    try { doc?.destroy(); } catch { /* noop */ }
    if (canvas) { canvas.width = 0; canvas.height = 0; }
  });

  // Re-render on page or zoom change (after first load).
  $effect(() => {
    pageIndex;
    zoom;
    if (doc) scheduleRender();
  });

  function clientToView(clientX: number, clientY: number) {
    const r = overlayEl.getBoundingClientRect();
    return { x: clientX - r.left, y: clientY - r.top };
  }
  const stage = $derived<StageInfo>({ ...dims, pageIndex, numPages, clientToView });
  $effect(() => { info = stage; });
  function go(delta: number) {
    pageIndex = Math.min(Math.max(pageIndex + delta, 0), numPages - 1);
  }
</script>

<div class="pdf-stage" bind:this={container}>
  {#if error}
    <div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="stage-error">
      <span class="ico"><Icon name="alert" /></span>
      <div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div>
    </div>
  {/if}

  <div class="pdf-stage-scroll">
    <div class="pdf-stage-wrap" style={dims.cssWidth ? `width:${dims.cssWidth}px;height:${dims.cssHeight}px` : ''}>
      <canvas bind:this={canvas} class="pdf-stage-canvas"></canvas>
      <div bind:this={overlayEl} class="pdf-stage-overlay">
        {#if overlay && dims.cssWidth}{@render overlay(stage)}{/if}
      </div>
      {#if loading}<div class="pdf-stage-loading"><span class="spinner"></span> Rendering…</div>{/if}
    </div>
  </div>

  {#if controls}
    <nav class="pdf-stage-pager" aria-label="Page navigation and zoom">
      {#if numPages > 1}
        <button class="btn btn-ghost btn-sm" type="button" onclick={() => go(-1)} disabled={pageIndex <= 0} aria-label="Previous page">‹ Prev</button>
        <span class="pager-label">Page {pageIndex + 1} of {numPages}</span>
        <button class="btn btn-ghost btn-sm" type="button" onclick={() => go(1)} disabled={pageIndex >= numPages - 1} aria-label="Next page">Next ›</button>
        <span class="pager-sep" aria-hidden="true"></span>
      {/if}
      <button class="btn btn-ghost btn-sm" type="button" onclick={() => setZoom(zoom - 0.25)} disabled={zoom <= ZOOM_MIN} aria-label="Zoom out" data-testid="zoom-out">−</button>
      <button class="btn btn-ghost btn-sm zoom-label" type="button" onclick={() => setZoom(1)} aria-label="Reset zoom to fit" data-testid="zoom-reset">{Math.round(zoom * 100)}%</button>
      <button class="btn btn-ghost btn-sm" type="button" onclick={() => setZoom(zoom + 0.25)} disabled={zoom >= ZOOM_MAX} aria-label="Zoom in" data-testid="zoom-in">+</button>
    </nav>
  {/if}
</div>

<style>
  .pdf-stage { display: flex; flex-direction: column; align-items: center; gap: 0.85rem; max-width: 100%; }
  /* Scroll container: when zoomed past the column width, the page pans here. */
  .pdf-stage-scroll { max-width: 100%; overflow: auto; border-radius: var(--radius-md); }
  .pdf-stage-wrap {
    position: relative;
    min-height: 220px; min-width: 200px;
    border-radius: var(--radius-md);
    overflow: hidden;
    box-shadow: var(--shadow-lg);
    background: var(--surface-inverse);
  }
  .pdf-stage-canvas { display: block; }
  /* pan-x/pan-y lets a zoomed page be scrolled by touch on empty areas; the
     interactive children (stamps, handles, crop rect) set touch-action:none. */
  .pdf-stage-overlay { position: absolute; inset: 0; touch-action: pan-x pan-y; }
  .pager-sep { width: 1px; height: 20px; background: var(--border); margin: 0 0.25rem; }
  .zoom-label { min-width: 64px; font-variant-numeric: tabular-nums; }
  .pdf-stage-loading {
    position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; gap: 0.5rem;
    color: var(--muted); font-size: 0.9rem; background: var(--surface-2);
  }
  .pdf-stage-pager { display: flex; align-items: center; gap: 0.5rem 0.85rem; flex-wrap: wrap; justify-content: center; }
  .pager-label { font-size: 0.875rem; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
