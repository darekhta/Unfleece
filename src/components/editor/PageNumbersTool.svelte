<script lang="ts">
  import type { Tool } from '../../lib/registry.js';
  import type { FriendlyError } from '../../lib/errors.js';
  import type { RunProgress } from '../../lib/progress.js';
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import Dropzone from '../Dropzone.svelte';
  import ProgressBlock from '../ProgressBlock.svelte';
  import PdfStage, { type StageInfo } from './PdfStage.svelte';
  import { downloadBlob, humanSize } from '../../lib/download.js';
  import { takePendingFiles } from '../../lib/handoff.js';
  import { friendlyError } from '../../lib/errors.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../../lib/focus.js';

  let { tool }: { tool: Tool } = $props();

  type Pos = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';
  const POSITIONS: Pos[] = ['top-left', 'top-center', 'top-right', 'bottom-left', 'bottom-center', 'bottom-right'];

  let files = $state<File[]>([]);
  let bytes = $state<Uint8Array | null>(null);

  let position = $state<Pos>('bottom-center');
  let format = $state('{n}');
  let startAt = $state(1);
  let fontSize = $state(12);
  let margin = $state(24);

  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('numbered.pdf');
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

  function numberFor(pageIndex: number, numPages: number): string {
    return format.replace(/\{n\}/g, String(pageIndex + startAt)).replace(/\{total\}/g, String(numPages));
  }
  function anchor(pos: Pos, m: number): string {
    const [v, h] = pos.split('-');
    let s = 'position:absolute;';
    s += v === 'top' ? `top:${m}px;` : `bottom:${m}px;`;
    if (h === 'left') s += `left:${m}px;`;
    else if (h === 'right') s += `right:${m}px;`;
    else s += 'left:50%;transform:translateX(-50%);';
    return s;
  }

  async function apply() {
    if (!bytes) return;
    status = 'working';
    error = null;
    resultBlob = null;
    progress = { phase: 'loading', label: 'Preparing page numbers…' };
    try {
      const { getPdfWorker, createProgressProxy } = await import('../../lib/worker/client.js');
      const out = await getPdfWorker().pageNumbers(
        bytes,
        { format, position, fontSize, margin, startAt },
        createProgressProxy((next) => (progress = next)),
      );
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = files[0].name.replace(/\.[^.]+$/, '') + '-numbered.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      error = friendlyError(e, "Couldn't add numbers");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      progress = null;
    }
  }
  function reset() {
    files = []; bytes = null; status = 'idle'; resultBlob = null; error = null; progress = null;
    void focusDropzoneAfterUpdate();
  }
</script>

<div class="runner-body">
  {#if !bytes}
    <Dropzone accept={tool.accept} multiple={false} bind:files />
  {:else if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head">
        <span class="tick"><Icon name="check" /></span>
        <div><h3>Page numbers added.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div>
      </div>
      <div class="download-row">
        <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(resultBlob!, resultName)}>
          <Icon name="download" /> {resultName} <span class="size">({humanSize(resultBlob.size)})</span>
        </button>
        <button class="btn btn-ghost" onclick={reset}>Start over</button>
      </div>
      <div class="made-here"><Icon name="shield" /> Made right here in your browser. We never saw it.</div>
    </div>
  {:else}
    <div class="editor-layout">
      {#key files[0]}
        <PdfStage {bytes}>
          {#snippet overlay(stage: StageInfo)}
            {#each POSITIONS as pos (pos)}
              <button
                type="button"
                class="pn-zone {position === pos ? 'sel' : ''}"
                style={anchor(pos, margin * stage.scale)}
                aria-pressed={position === pos}
                aria-label={`Place numbers ${pos.replace('-', ' ')}`}
                onclick={() => (position = pos)}
              >
                <span class="pn-num" style={`font-size:${Math.max(9, fontSize * stage.scale)}px`}>
                  {position === pos ? numberFor(stage.pageIndex, stage.numPages) : '#'}
                </span>
              </button>
            {/each}
          {/snippet}
        </PdfStage>
      {/key}

      <div class="options-panel editor-controls">
        <h3>Numbering</h3>
        <label class="label" for="pn-format">Format</label>
        <input id="pn-format" class="field" bind:value={format} placeholder="{'{n}'}" />
        <div class="preset-row">
          {#each [['{n}', '1'], ['Page {n}', 'Page 1'], ['{n} of {total}', '1 of N'], ['- {n} -', '- 1 -']] as [fmt, lbl] (fmt)}
            <button type="button" class="preset {format === fmt ? 'on' : ''}" onclick={() => (format = fmt)}>{lbl}</button>
          {/each}
        </div>
        <p class="help">Tap a position on the page. <code>{'{n}'}</code> = number, <code>{'{total}'}</code> = total.</p>

        <div class="ctl-grid">
          <div><label class="label" for="pn-start">Start at</label><input id="pn-start" class="field" type="number" min="0" bind:value={startAt} /></div>
          <div><label class="label" for="pn-size">Font size</label><input id="pn-size" class="field" type="number" min="6" max="48" bind:value={fontSize} /></div>
          <div><label class="label" for="pn-margin">Margin (pt)</label><input id="pn-margin" class="field" type="number" min="0" max="120" bind:value={margin} /></div>
        </div>

        {#if status === 'error' && error}
          <div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error"><span class="ico"><Icon name="alert" /></span><div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div></div>
        {/if}

        {#if status === 'working'}
          <ProgressBlock {progress} fallback="Adding page numbers…" />
        {/if}

        <div class="action-row" style="margin-top:1rem">
          {#if status === 'working'}
            <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Working…</button>
          {:else}
            <button class="btn btn-primary" data-testid="run-button" onclick={apply}>Add page numbers <Icon name="arrowRight" sw={2} /></button>
          {/if}
          <button class="btn btn-ghost" onclick={reset}>Choose another file</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 1.5rem; align-items: start; }
  @media (max-width: 820px) { .editor-layout { grid-template-columns: 1fr; } }
  .editor-controls { align-self: start; }
  .pn-zone {
    min-width: 44px; min-height: 44px; padding: 0.2rem 0.4rem;
    display: grid; place-items: center; cursor: pointer;
    background: rgba(46, 204, 143, 0.06); border: 1px dashed rgba(46, 204, 143, 0.5); border-radius: var(--radius-sm);
    color: #0b1220; transition: background var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  }
  .pn-zone:hover { background: rgba(46, 204, 143, 0.14); }
  .pn-zone.sel { background: rgba(46, 204, 143, 0.18); border-style: solid; border-color: var(--accent-500); box-shadow: var(--ring-accent-glow); }
  .pn-num { font-weight: 600; line-height: 1; font-family: var(--font-sans); }
  .pn-zone:not(.sel) .pn-num { color: var(--accent-700); opacity: 0.6; }
  .preset-row { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.5rem; }
  .preset { font-size: 0.8rem; padding: 0.3rem 0.6rem; border-radius: var(--radius-full); border: 1px solid var(--border); background: var(--surface-glass-strong); color: var(--ink-soft); cursor: pointer; }
  .preset.on { border-color: var(--border-accent); color: var(--ink); background: var(--surface-glass); }
  .ctl-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 0.75rem; margin-top: 0.85rem; }
  code { font-family: var(--font-mono); font-size: 0.85em; }
</style>
