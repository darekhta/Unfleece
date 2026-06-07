<script lang="ts">
  import type { Tool } from '../../lib/registry.js';
  import type { FriendlyError } from '../../lib/errors.js';
  import type { RunProgress } from '../../lib/progress.js';
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Dropzone from '../Dropzone.svelte';
  import ProgressBlock from '../ProgressBlock.svelte';
  import PdfStage, { type StageInfo } from './PdfStage.svelte';
  import Handle from './Handle.svelte';
  import { startDrag } from '../../lib/browser/useDrag.js';
  import { downloadBlob, humanSize } from '../../lib/download.js';
  import { takePendingFiles } from '../../lib/handoff.js';
  import { friendlyError } from '../../lib/errors.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../../lib/focus.js';

  let { tool }: { tool: Tool } = $props();

  let files = $state<File[]>([]);
  let bytes = $state<Uint8Array | null>(null);
  let stageInfo = $state<StageInfo>();

  // Crop rect as fractions of the page (dimension-independent → survives resize).
  let rect = $state({ x: 0.06, y: 0.06, w: 0.88, h: 0.88 });
  const MIN = 0.06;

  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('cropped.pdf');
  let progress = $state<RunProgress | null>(null);
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;

  onMount(async () => {
    try {
      const p = await takePendingFiles();
      if (p && p.length) files = [p[0]];
    } catch (e) { /* ignore */ }
  });
  $effect(() => {
    const f = files[0];
    if (!f) { bytes = null; return; }
    f.arrayBuffer().then((b) => (bytes = new Uint8Array(b)));
  });

  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

  function resize(edges: string, dxF: number, dyF: number) {
    let l = rect.x, t = rect.y, r = rect.x + rect.w, b = rect.y + rect.h;
    if (edges.includes('L')) l = clamp(l + dxF, 0, r - MIN);
    if (edges.includes('R')) r = clamp(r + dxF, l + MIN, 1);
    if (edges.includes('T')) t = clamp(t + dyF, 0, b - MIN);
    if (edges.includes('B')) b = clamp(b + dyF, t + MIN, 1);
    rect = { x: l, y: t, w: r - l, h: b - t };
  }
  function move(dxF: number, dyF: number) {
    rect = { ...rect, x: clamp(rect.x + dxF, 0, 1 - rect.w), y: clamp(rect.y + dyF, 0, 1 - rect.h) };
  }
  function bodyDown(e: PointerEvent, s: StageInfo) {
    startDrag(e, { onMove: (dx, dy) => move(dx / s.cssWidth, dy / s.cssHeight) });
  }

  // Live margins in points (for readout + numeric inputs).
  const margins = $derived.by(() => {
    const s = stageInfo;
    if (!s) return { top: 0, right: 0, bottom: 0, left: 0, w: 0, h: 0 };
    return {
      top: rect.y * s.pageHeightPt,
      bottom: (1 - rect.y - rect.h) * s.pageHeightPt,
      left: rect.x * s.pageWidthPt,
      right: (1 - rect.x - rect.w) * s.pageWidthPt,
      w: rect.w * s.pageWidthPt,
      h: rect.h * s.pageHeightPt,
    };
  });
  function setMargin(which: 'top' | 'right' | 'bottom' | 'left', ptStr: string) {
    const s = stageInfo;
    if (!s) return;
    const pt = Math.max(0, Number(ptStr) || 0);
    const m = { top: margins.top, right: margins.right, bottom: margins.bottom, left: margins.left };
    m[which] = pt;
    const x = clamp(m.left / s.pageWidthPt, 0, 1 - MIN);
    const w = clamp(1 - m.left / s.pageWidthPt - m.right / s.pageWidthPt, MIN, 1 - x);
    const y = clamp(m.top / s.pageHeightPt, 0, 1 - MIN);
    const h = clamp(1 - m.top / s.pageHeightPt - m.bottom / s.pageHeightPt, MIN, 1 - y);
    rect = { x, y, w, h };
  }

  const HANDLES: { k: string; e: string; pos: (l: number, t: number, r: number, b: number) => [number, number] }[] = [
    { k: 'nw', e: 'LT', pos: (l, t, r, b) => [l, t] },
    { k: 'n', e: 'T', pos: (l, t, r, b) => [(l + r) / 2, t] },
    { k: 'ne', e: 'RT', pos: (l, t, r, b) => [r, t] },
    { k: 'e', e: 'R', pos: (l, t, r, b) => [r, (t + b) / 2] },
    { k: 'se', e: 'RB', pos: (l, t, r, b) => [r, b] },
    { k: 's', e: 'B', pos: (l, t, r, b) => [(l + r) / 2, b] },
    { k: 'sw', e: 'LB', pos: (l, t, r, b) => [l, b] },
    { k: 'w', e: 'L', pos: (l, t, r, b) => [l, (t + b) / 2] },
  ];

  async function apply() {
    if (!bytes) return;
    status = 'working';
    error = null;
    resultBlob = null;
    progress = { phase: 'loading', label: 'Preparing crop…' };
    try {
      const { getPdfWorker, createProgressProxy } = await import('../../lib/worker/client.js');
      const out = await getPdfWorker().crop(bytes, {
        top: Math.max(0, margins.top), right: Math.max(0, margins.right),
        bottom: Math.max(0, margins.bottom), left: Math.max(0, margins.left),
      }, createProgressProxy((next) => (progress = next)));
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = files[0].name.replace(/\.[^.]+$/, '') + '-cropped.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      error = friendlyError(e, "Couldn't crop");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      progress = null;
    }
  }
  function reset() { files = []; bytes = null; status = 'idle'; resultBlob = null; error = null; progress = null; rect = { x: 0.06, y: 0.06, w: 0.88, h: 0.88 }; void focusDropzoneAfterUpdate(); }
</script>

<div class="runner-body">
  {#if !bytes}
    <Dropzone accept={tool.accept} multiple={false} bind:files />
  {:else if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head"><span class="tick"><Icon name="check" /></span><div><h3>Cropped.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div></div>
      <div class="download-row">
        <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(resultBlob!, resultName)}><Icon name="download" /> {resultName} <span class="size">({humanSize(resultBlob.size)})</span></button>
        <button class="btn btn-ghost" onclick={reset}>Start over</button>
      </div>
      <div class="made-here"><Icon name="shield" /> Made right here in your browser. We never saw it.</div>
    </div>
  {:else}
    <div class="editor-layout">
      {#key files[0]}
        <PdfStage {bytes} bind:info={stageInfo}>
          {#snippet overlay(s: StageInfo)}
            {@const lx = rect.x * s.cssWidth}
            {@const ty = rect.y * s.cssHeight}
            {@const rw = rect.w * s.cssWidth}
            {@const rh = rect.h * s.cssHeight}
            <!-- dimmed scrim around the keep-area -->
            <div class="scrim" style={`left:0;top:0;width:100%;height:${ty}px`}></div>
            <div class="scrim" style={`left:0;top:${ty + rh}px;width:100%;height:${s.cssHeight - ty - rh}px`}></div>
            <div class="scrim" style={`left:0;top:${ty}px;width:${lx}px;height:${rh}px`}></div>
            <div class="scrim" style={`left:${lx + rw}px;top:${ty}px;width:${s.cssWidth - lx - rw}px;height:${rh}px`}></div>
            <!-- crop rect (body drag) -->
            <div
              class="crop-rect"
              style={`left:${lx}px;top:${ty}px;width:${rw}px;height:${rh}px`}
              onpointerdown={(e) => bodyDown(e, s)}
              role="application"
              aria-label="Crop area — drag to move"
            ></div>
            {#each HANDLES as h (h.k)}
              {@const [hx, hy] = h.pos(lx, ty, lx + rw, ty + rh)}
              <Handle
                variant={h.k.length === 2 ? 'corner' : 'edge'}
                label={`Resize ${h.k}`}
                style={`left:${hx}px;top:${hy}px;transform:translate(-50%,-50%)`}
                onmove={(dx, dy) => resize(h.e, dx / s.cssWidth, dy / s.cssHeight)}
              />
            {/each}
          {/snippet}
        </PdfStage>
      {/key}

      <div class="options-panel editor-controls">
        <h3>Crop margins (pt)</h3>
        <p class="help">Drag the box or its handles. Applies to all pages.</p>
        <div class="ctl-grid">
          <div><label class="label" for="cr-t">Top</label><input id="cr-t" class="field" type="number" min="0" value={margins.top.toFixed(0)} onchange={(e) => setMargin('top', (e.target as HTMLInputElement).value)} /></div>
          <div><label class="label" for="cr-r">Right</label><input id="cr-r" class="field" type="number" min="0" value={margins.right.toFixed(0)} onchange={(e) => setMargin('right', (e.target as HTMLInputElement).value)} /></div>
          <div><label class="label" for="cr-b">Bottom</label><input id="cr-b" class="field" type="number" min="0" value={margins.bottom.toFixed(0)} onchange={(e) => setMargin('bottom', (e.target as HTMLInputElement).value)} /></div>
          <div><label class="label" for="cr-l">Left</label><input id="cr-l" class="field" type="number" min="0" value={margins.left.toFixed(0)} onchange={(e) => setMargin('left', (e.target as HTMLInputElement).value)} /></div>
        </div>
        <p class="readout mono">{margins.w.toFixed(0)} × {margins.h.toFixed(0)} pt</p>

        {#if status === 'error' && error}<div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error"><span class="ico"><Icon name="alert" /></span><div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div></div>{/if}

        {#if status === 'working'}
          <ProgressBlock {progress} fallback="Cropping PDF…" />
        {/if}

        <div class="action-row" style="margin-top:1rem">
          {#if status === 'working'}
            <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Working…</button>
          {:else}
            <button class="btn btn-primary" data-testid="run-button" onclick={apply}>Crop PDF <Icon name="arrowRight" sw={2} /></button>
          {/if}
          <button class="btn btn-ghost" onclick={reset}>Choose another file</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-layout { display: grid; grid-template-columns: minmax(0, 1fr) 300px; gap: 1.5rem; align-items: start; }
  @media (max-width: 820px) { .editor-layout { grid-template-columns: 1fr; } }
  .scrim { position: absolute; background: rgba(7, 9, 15, 0.5); pointer-events: none; }
  .crop-rect { position: absolute; border: 2px solid var(--accent-500); box-shadow: var(--ring-accent-glow); cursor: move; touch-action: none; z-index: 2; }
  .ctl-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
  .readout { margin-top: 0.75rem; color: var(--accent-700); font-weight: 600; }
</style>
