<script lang="ts">
  import type { Tool } from '../lib/registry.js';
  import type { FriendlyError } from '../lib/errors.js';
  import type { RunProgress } from '../lib/progress.js';
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import Dropzone from './Dropzone.svelte';
  import ProgressBlock from './ProgressBlock.svelte';
  import PdfStage, { type StageInfo } from './editor/PdfStage.svelte';
  import Handle from './editor/Handle.svelte';
  import { startDrag } from '../lib/browser/useDrag.js';
  import { downloadBlob, humanSize, outputBaseName } from '../lib/download.js';
  import { takePendingFiles } from '../lib/handoff.js';
  import { friendlyError } from '../lib/errors.js';
  import { reportError, engineForTool } from '../lib/telemetry.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../lib/focus.js';

  let { tool }: { tool: Tool } = $props();

  interface RedactionBox {
    id: string;
    page: number;
    x: number;
    y: number;
    w: number;
    h: number;
  }

  let files = $state<File[]>([]);
  let bytes = $state<Uint8Array | null>(null);
  let stageInfo = $state<StageInfo>();
  let pageIndex = $state(0);
  let boxes = $state<RedactionBox[]>([]);
  let draft = $state<RedactionBox | null>(null);
  let activeId = $state<string | null>(null);
  let color = $state<'black' | 'white'>('black');
  let scale = $state(2);

  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('redacted.pdf');
  let progress = $state<RunProgress | null>(null);
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;

  const MIN = 0.025;
  const uid = () =>
    typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : `r-${Math.floor(performance.now() * 1000)}`;
  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

  onMount(async () => {
    try { const pending = await takePendingFiles(); if (pending && pending.length) files = [pending[0]]; } catch { /* ignore */ }
  });

  $effect(() => {
    const f = files[0];
    if (!f) { bytes = null; return; }
    f.arrayBuffer().then((buffer) => (bytes = new Uint8Array(buffer)));
  });

  const pagesTouched = $derived([...new Set(boxes.map((box) => box.page + 1))].sort((a, b) => a - b));
  const selected = $derived(boxes.find((box) => box.id === activeId) ?? null);

  function boxesForPage(page: number): RedactionBox[] {
    return boxes.filter((box) => box.page === page);
  }

  function normalizeBox(page: number, ax: number, ay: number, bx: number, by: number): RedactionBox {
    const x1 = clamp(Math.min(ax, bx), 0, 1);
    const y1 = clamp(Math.min(ay, by), 0, 1);
    const x2 = clamp(Math.max(ax, bx), 0, 1);
    const y2 = clamp(Math.max(ay, by), 0, 1);
    return { id: draft?.id ?? uid(), page, x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
  }

  function drawDown(e: PointerEvent, s: StageInfo) {
    const start = s.clientToView(e.clientX, e.clientY);
    const sx = clamp(start.x / s.cssWidth, 0, 1);
    const sy = clamp(start.y / s.cssHeight, 0, 1);
    const id = uid();
    draft = { id, page: s.pageIndex, x: sx, y: sy, w: 0, h: 0 };
    startDrag(e, {
      onMove: (_dx, _dy, ev) => {
        const next = s.clientToView(ev.clientX, ev.clientY);
        const nextBox = normalizeBox(s.pageIndex, sx, sy, next.x / s.cssWidth, next.y / s.cssHeight);
        draft = { ...nextBox, id };
      },
      onEnd: () => {
        const d = draft;
        draft = null;
        if (!d || d.w < MIN || d.h < MIN) return;
        boxes = [...boxes, d];
        activeId = d.id;
        if (status === 'error') { error = null; status = 'idle'; }
      },
    });
  }

  function updateBox(id: string, update: (box: RedactionBox) => RedactionBox) {
    boxes = boxes.map((box) => (box.id === id ? update(box) : box));
    activeId = id;
  }

  function moveBox(id: string, dxF: number, dyF: number) {
    updateBox(id, (box) => ({ ...box, x: clamp(box.x + dxF, 0, 1 - box.w), y: clamp(box.y + dyF, 0, 1 - box.h) }));
  }

  function resizeBox(id: string, edges: string, dxF: number, dyF: number) {
    updateBox(id, (box) => {
      let l = box.x;
      let t = box.y;
      let r = box.x + box.w;
      let b = box.y + box.h;
      if (edges.includes('L')) l = clamp(l + dxF, 0, r - MIN);
      if (edges.includes('R')) r = clamp(r + dxF, l + MIN, 1);
      if (edges.includes('T')) t = clamp(t + dyF, 0, b - MIN);
      if (edges.includes('B')) b = clamp(b + dyF, t + MIN, 1);
      return { ...box, x: l, y: t, w: r - l, h: b - t };
    });
  }

  function boxDown(e: PointerEvent, box: RedactionBox, s: StageInfo) {
    activeId = box.id;
    startDrag(e, { onMove: (dx, dy) => moveBox(box.id, dx / s.cssWidth, dy / s.cssHeight) });
  }

  function removeSelected() {
    if (!activeId) return;
    boxes = boxes.filter((box) => box.id !== activeId);
    activeId = null;
  }

  function clearPage() {
    boxes = boxes.filter((box) => box.page !== pageIndex);
    if (selected?.page === pageIndex) activeId = null;
  }

  function clearAll() {
    boxes = [];
    activeId = null;
  }

  const HANDLES: { key: string; edges: string; pos: (l: number, t: number, r: number, b: number) => [number, number] }[] = [
    { key: 'nw', edges: 'LT', pos: (l, t) => [l, t] },
    { key: 'n', edges: 'T', pos: (l, t, r) => [(l + r) / 2, t] },
    { key: 'ne', edges: 'RT', pos: (_l, t, r) => [r, t] },
    { key: 'e', edges: 'R', pos: (_l, t, r, b) => [r, (t + b) / 2] },
    { key: 'se', edges: 'RB', pos: (_l, _t, r, b) => [r, b] },
    { key: 's', edges: 'B', pos: (l, _t, r, b) => [(l + r) / 2, b] },
    { key: 'sw', edges: 'LB', pos: (l, _t, _r, b) => [l, b] },
    { key: 'w', edges: 'L', pos: (l, t, _r, b) => [l, (t + b) / 2] },
  ];

  async function apply() {
    if (!bytes) return;
    if (boxes.length === 0) {
      error = { title: 'Nothing to redact', message: 'Draw at least one box over content before running redaction.' };
      status = 'error';
      await focusAfterUpdate(() => errorEl);
      return;
    }

    status = 'working';
    error = null;
    resultBlob = null;
    progress = { phase: 'rendering', label: 'Preparing redaction…' };
    try {
      const { redactPdf } = await import('../lib/browser/redact.js');
      const out = await redactPdf(bytes, {
        redactions: boxes.map((box) => ({ pageIndex: box.page, x: box.x, y: box.y, w: box.w, h: box.h })),
        color,
        scale,
      }, { onProgress: (next) => (progress = next) });
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = outputBaseName(files[0].name) + '-redacted.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      reportError(tool.id, e, engineForTool(tool.id));
      error = friendlyError(e, "Couldn't redact this PDF");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      progress = null;
    }
  }

  function reset() {
    files = [];
    bytes = null;
    boxes = [];
    draft = null;
    activeId = null;
    pageIndex = 0;
    status = 'idle';
    error = null;
    resultBlob = null;
    progress = null;
    void focusDropzoneAfterUpdate();
  }
</script>

<div class="runner-body">
  {#if !bytes}
    <Dropzone accept={tool.accept} multiple={false} bind:files />
  {:else if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head"><span class="tick"><Icon name="check" /></span><div><h3>Redacted.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div></div>
      <div class="download-row">
        <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(resultBlob!, resultName)}><Icon name="download" /> {resultName} <span class="size">({humanSize(resultBlob.size)})</span></button>
        <button class="btn btn-ghost" onclick={reset}>Start over</button>
      </div>
      <div class="made-here"><Icon name="shield" /> Made right here in your browser. We never saw it.</div>
    </div>
  {:else}
    <div class="editor-layout">
      {#key files[0]}
        <PdfStage {bytes} bind:pageIndex bind:info={stageInfo}>
          {#snippet overlay(s: StageInfo)}
            <div class="redact-overlay" data-testid="redact-overlay" onpointerdown={(e) => drawDown(e, s)}></div>

            {#each boxesForPage(s.pageIndex) as box (box.id)}
              {@const l = box.x * s.cssWidth}
              {@const t = box.y * s.cssHeight}
              {@const w = box.w * s.cssWidth}
              {@const h = box.h * s.cssHeight}
              <button
                type="button"
                class={`redact-box ${box.id === activeId ? 'active' : ''} ${color === 'white' ? 'white' : ''}`}
                style={`left:${l}px;top:${t}px;width:${w}px;height:${h}px`}
                aria-label="Redaction box"
                onpointerdown={(e) => boxDown(e, box, s)}
              ></button>
              {#if box.id === activeId}
                {#each HANDLES as handle (handle.key)}
                  {@const [hx, hy] = handle.pos(l, t, l + w, t + h)}
                  <Handle
                    variant={handle.key.length === 2 ? 'corner' : 'edge'}
                    label={`Resize redaction ${handle.key}`}
                    style={`left:${hx}px;top:${hy}px;transform:translate(-50%,-50%)`}
                    onmove={(dx, dy) => resizeBox(box.id, handle.edges, dx / s.cssWidth, dy / s.cssHeight)}
                  />
                {/each}
              {/if}
            {/each}

            {#if draft && draft.page === s.pageIndex}
              <div
                class={`redact-box draft ${color === 'white' ? 'white' : ''}`}
                style={`left:${draft.x * s.cssWidth}px;top:${draft.y * s.cssHeight}px;width:${draft.w * s.cssWidth}px;height:${draft.h * s.cssHeight}px`}
              ></div>
            {/if}
          {/snippet}
        </PdfStage>
      {/key}

      <div class="options-panel editor-controls">
        <h3>Redaction boxes</h3>
        <p class="help">Draw boxes over content to remove. Output pages are rebuilt as images.</p>

        <div class="redact-summary" data-testid="redaction-count">
          <strong>{boxes.length}</strong>
          <span>{boxes.length === 1 ? 'box' : 'boxes'}{pagesTouched.length ? ` · pages ${pagesTouched.join(', ')}` : ''}</span>
        </div>

        <div class="ctl-grid">
          <div>
            <label class="label" for="redact-color">Fill</label>
            <select id="redact-color" class="field" bind:value={color}>
              <option value="black">Black</option>
              <option value="white">White</option>
            </select>
          </div>
          <div>
            <label class="label" for="redact-scale">Render scale</label>
            <input id="redact-scale" class="field" type="number" min="1" max="3" step="0.25" bind:value={scale} />
          </div>
        </div>

        <div class="action-row compact-actions">
          <button class="btn btn-ghost btn-sm" type="button" onclick={removeSelected} disabled={!activeId}><Icon name="x" /> Selected</button>
          <button class="btn btn-ghost btn-sm" type="button" onclick={clearPage} disabled={boxesForPage(pageIndex).length === 0}>Clear page</button>
          <button class="btn btn-ghost btn-sm" type="button" onclick={clearAll} disabled={boxes.length === 0}>Clear all</button>
        </div>

        {#if status === 'error' && error}<div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error"><span class="ico"><Icon name="alert" /></span><div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div></div>{/if}

        {#if status === 'working'}
          <ProgressBlock {progress} fallback="Redacting PDF…" />
        {/if}

        <div class="action-row" style="margin-top:1rem">
          {#if status === 'working'}
            <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Working…</button>
          {:else}
            <button class="btn btn-primary" data-testid="run-button" disabled={boxes.length === 0} onclick={apply}>Redact PDF <Icon name="arrowRight" sw={2} /></button>
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
  .redact-overlay { position: absolute; inset: 0; z-index: 1; cursor: crosshair; touch-action: none; }
  .redact-box {
    position: absolute;
    z-index: 2;
    padding: 0;
    border: 2px solid #111827;
    border-radius: 2px;
    background: rgba(0, 0, 0, 0.82);
    box-shadow: 0 0 0 1px rgba(255,255,255,.7);
    cursor: move;
    touch-action: none;
  }
  .redact-box.white {
    border-color: #111827;
    background: rgba(255, 255, 255, 0.9);
  }
  .redact-box.active { border-color: var(--accent-500); box-shadow: var(--ring-accent-glow); }
  .redact-box.draft { pointer-events: none; opacity: 0.78; }
  .redact-summary {
    display: flex; align-items: baseline; gap: .5rem;
    margin: .8rem 0 1rem;
    color: var(--muted);
  }
  .redact-summary strong { color: var(--ink); font-size: 1.4rem; }
  .ctl-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
  .compact-actions { gap: .5rem; flex-wrap: wrap; margin-top: .85rem; }
</style>
