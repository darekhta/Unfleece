<script lang="ts">
  import type { Tool } from '../../lib/registry.js';
  import type { FriendlyError } from '../../lib/errors.js';
  import type { RunProgress } from '../../lib/progress.js';
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Dropzone from '../Dropzone.svelte';
  import ProgressBlock from '../ProgressBlock.svelte';
  import PdfStage, { type StageInfo } from './PdfStage.svelte';
  import { downloadBlob, humanSize, outputBaseName } from '../../lib/download.js';
  import { takePendingFiles } from '../../lib/handoff.js';
  import { friendlyError } from '../../lib/errors.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../../lib/focus.js';

  let { tool }: { tool: Tool } = $props();

  let files = $state<File[]>([]);
  let bytes = $state<Uint8Array | null>(null);

  let text = $state('CONFIDENTIAL');
  let fontSize = $state(50);
  let opacity = $state(0.25);
  let angle = $state(45);

  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('watermarked.pdf');
  let progress = $state<RunProgress | null>(null);
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;

  onMount(async () => {
    try { const p = await takePendingFiles(); if (p && p.length) files = [p[0]]; } catch (e) { /* ignore */ }
  });
  $effect(() => {
    const f = files[0];
    if (!f) { bytes = null; return; }
    f.arrayBuffer().then((b) => (bytes = new Uint8Array(b)));
  });

  async function apply() {
    if (!bytes || !text) return;
    status = 'working';
    error = null;
    resultBlob = null;
    progress = { phase: 'loading', label: 'Preparing watermark…' };
    try {
      const { getPdfWorker, createProgressProxy } = await import('../../lib/worker/client.js');
      const out = await getPdfWorker().watermark(
        bytes,
        { text, fontSize, opacity, angle },
        createProgressProxy((next) => (progress = next)),
      );
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = outputBaseName(files[0].name) + '-watermarked.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      error = friendlyError(e, "Couldn't add watermark");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      progress = null;
    }
  }
  function reset() { files = []; bytes = null; status = 'idle'; resultBlob = null; error = null; progress = null; void focusDropzoneAfterUpdate(); }
</script>

<div class="runner-body">
  {#if !bytes}
    <Dropzone accept={tool.accept} multiple={false} bind:files />
  {:else if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head"><span class="tick"><Icon name="check" /></span><div><h3>Watermark added.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div></div>
      <div class="download-row">
        <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(resultBlob!, resultName)}><Icon name="download" /> {resultName} <span class="size">({humanSize(resultBlob.size)})</span></button>
        <button class="btn btn-ghost" onclick={reset}>Start over</button>
      </div>
      <div class="made-here"><Icon name="shield" /> Made right here in your browser. We never saw it.</div>
    </div>
  {:else}
    <div class="editor-layout">
      {#key files[0]}
        <PdfStage {bytes}>
          {#snippet overlay(s: StageInfo)}
            <div
              class="wm"
              style={`opacity:${opacity}; font-size:${Math.max(10, fontSize * s.scale)}px; transform:translate(-50%,-50%) rotate(${-angle}deg)`}
            >{text}</div>
          {/snippet}
        </PdfStage>
      {/key}

      <div class="options-panel editor-controls">
        <h3>Watermark</h3>
        <label class="label" for="wm-text">Text</label>
        <input id="wm-text" class="field" bind:value={text} />
        <label class="label" for="wm-op" style="margin-top:.75rem">Opacity — {Math.round(opacity * 100)}%</label>
        <input id="wm-op" type="range" min="0.05" max="1" step="0.05" bind:value={opacity} class="range" />
        <label class="label" for="wm-ang" style="margin-top:.5rem">Angle — {angle}°</label>
        <input id="wm-ang" type="range" min="-90" max="90" step="1" bind:value={angle} class="range" />
        <label class="label" for="wm-size" style="margin-top:.5rem">Size — {fontSize}pt</label>
        <input id="wm-size" type="range" min="12" max="160" step="2" bind:value={fontSize} class="range" />

        {#if status === 'error' && error}<div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error"><span class="ico"><Icon name="alert" /></span><div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div></div>{/if}

        {#if status === 'working'}
          <ProgressBlock {progress} fallback="Adding watermark…" />
        {/if}

        <div class="action-row" style="margin-top:1rem">
          {#if status === 'working'}
            <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Working…</button>
          {:else}
            <button class="btn btn-primary" data-testid="run-button" disabled={!text} onclick={apply}>Add watermark <Icon name="arrowRight" sw={2} /></button>
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
  .wm {
    position: absolute; left: 50%; top: 50%;
    font-weight: 700; color: rgb(153, 153, 153); white-space: nowrap; pointer-events: none;
    font-family: var(--font-sans); user-select: none;
  }
  .range { width: 100%; accent-color: var(--accent-500); }
</style>
