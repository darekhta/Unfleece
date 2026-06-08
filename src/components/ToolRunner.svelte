<script lang="ts">
  import type { Tool } from '../lib/registry.js';
  import type { RunResult } from '../lib/run.js';
  import type { RunProgress } from '../lib/progress.js';
  import type { FriendlyError } from '../lib/errors.js';
  import type { RunnerText } from '../lib/i18n/types.js';
  import Icon from './Icon.svelte';
  import Dropzone from './Dropzone.svelte';
  import OptionsForm from './OptionsForm.svelte';
  import ProgressBlock from './ProgressBlock.svelte';
  import { downloadBlob, humanSize } from '../lib/download.js';
  import { onMount } from 'svelte';
  import { takePendingFiles, fileMatchesAccept } from '../lib/handoff.js';
  import { friendlyError } from '../lib/errors.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../lib/focus.js';

  let { tool, t }: { tool: Tool; t: RunnerText } = $props();

  function initialValues(): Record<string, unknown> {
    const v: Record<string, unknown> = {};
    for (const f of tool.options ?? []) v[f.name] = 'default' in f && f.default !== undefined ? f.default : '';
    return v;
  }

  let files = $state<File[]>([]);
  let values = $state<Record<string, unknown>>(initialValues());
  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let result = $state<RunResult | null>(null);
  let progress = $state<RunProgress | null>(null);
  let pageCount = $state<number | null>(null);
  let optionsValid = $state(true);
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;
  let pageCountVersion = 0;
  let abortController: AbortController | null = null;

  const LARGE_FILE_BYTES = 75 * 1024 * 1024;
  const LARGE_TOTAL_BYTES = 150 * 1024 * 1024;
  const totalBytes = $derived(files.reduce((sum, f) => sum + f.size, 0));
  const hasLargeFiles = $derived(files.some((f) => f.size >= LARGE_FILE_BYTES) || totalBytes >= LARGE_TOTAL_BYTES);
  const needsPageCount = $derived(Boolean(tool.options?.some((f) =>
    f.type === 'pages' || f.name === 'ranges' || f.name === 'order',
  )));

  // Pick up a file handed over from the hero quick-start (if it fits this tool).
  onMount(async () => {
    try {
      const pending = await takePendingFiles();
      if (!pending) return;
      const ok = pending.filter((f) => fileMatchesAccept(f, tool.accept));
      if (ok.length) files = tool.multiple ? ok : ok.slice(0, 1);
    } catch (e) {
      /* ignore */
    }
  });

  $effect(() => {
    const file = files[0];
    const version = ++pageCountVersion;
    pageCount = null;
    if (!file || !needsPageCount) return;
    (async () => {
      try {
        const { getPdfWorker } = await import('../lib/worker/client.js');
        const bytes = new Uint8Array(await file.arrayBuffer());
        const count = await getPdfWorker().getPageCount(bytes);
        if (version === pageCountVersion) pageCount = count;
      } catch {
        if (version === pageCountVersion) pageCount = null;
      }
    })();
  });

  const stepActive = $derived(status === 'done' ? 3 : status === 'working' ? 2 : files.length > 0 ? 1 : 0);
  const STEPS = $derived([t.stepAdd, t.stepOptions, t.stepRun, t.stepDownload]);

  async function run() {
    abortController = new AbortController();
    status = 'working';
    error = null;
    result = null;
    progress = { phase: 'loading', label: t.progressFallback };
    const controller = abortController;
    try {
      const { runTool } = await import('../lib/run.js');
      result = await runTool(tool, files, values, {
        signal: controller.signal,
        onProgress: (next) => {
          if (abortController === controller) progress = next;
        },
      });
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      if (e instanceof DOMException && e.name === 'AbortError') {
        status = 'idle';
        return;
      }
      error = friendlyError(e);
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      if (abortController === controller) abortController = null;
      progress = null;
    }
  }
  function cancel() {
    abortController?.abort();
  }
  function reset() {
    abortController?.abort();
    abortController = null;
    files = [];
    values = initialValues();
    status = 'idle';
    error = null;
    result = null;
    progress = null;
    pageCount = null;
    optionsValid = true;
    void focusDropzoneAfterUpdate();
  }
  async function copyText() {
    if (result?.text) {
      try { await navigator.clipboard.writeText(result.text); } catch (e) { /* ignore */ }
    }
  }
</script>

{#if tool.special}
  <div class="alert alert-error" role="alert" data-testid="error">
    <span class="ico"><Icon name="alert" /></span>
    <div class="a-body">
      <strong>{t.setupErrorTitle}</strong>
      <p>{t.setupErrorBody.replace('{tool}', tool.name)}</p>
    </div>
  </div>
{:else}
  <!-- Steps -->
  <div class="runner-steps" aria-hidden="true">
    {#each STEPS as label, i (label)}
      <span class="s {i <= stepActive ? 'on' : ''}"><span class="num">{i + 1}</span>{label}</span>
      {#if i < STEPS.length - 1}<span class="arrow"><Icon name="chevronRight" /></span>{/if}
    {/each}
  </div>

  <div class="runner-body">
    {#if status !== 'done'}
      <Dropzone accept={tool.accept} multiple={tool.multiple} bind:files {t} />

      {#if hasLargeFiles}
        <div class="alert alert-warn" role="status" data-testid="large-file-warning">
          <span class="ico"><Icon name="alert" /></span>
          <div class="a-body">
            <strong>{t.largeTitle}</strong>
            <p>{t.largeBody}</p>
          </div>
        </div>
      {/if}

      {#if files.length > 0 && tool.options && tool.options.length > 0}
        <OptionsForm fields={tool.options} bind:values bind:valid={optionsValid} {pageCount} />
      {/if}

      {#if status === 'working'}
        <ProgressBlock {progress} fallback={t.progressFallback} />
      {/if}

      {#if status === 'error' && error}
        <div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error">
          <span class="ico"><Icon name="alert" /></span>
          <div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div>
        </div>
      {/if}

      <div class="action-row">
        {#if status === 'working'}
          <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> {t.working}</button>
          <button class="btn btn-ghost" type="button" data-testid="cancel-button" onclick={cancel}>{t.cancel}</button>
        {:else}
          <button class="btn btn-primary" data-testid="run-button" disabled={files.length === 0 || !optionsValid} onclick={run}>
            {tool.name} <Icon name="arrowRight" sw={2} />
          </button>
        {/if}
        {#if status === 'error'}<button class="btn btn-ghost" onclick={reset}>{t.startOver}</button>{/if}
        {#if files.length === 0}<span class="action-hint">{t.addFileFirst}</span>{:else if !optionsValid}<span class="action-hint">{t.fixOptions}</span>{/if}
      </div>
    {/if}

    {#if status === 'done' && result}
      <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
        <div class="result-head">
          <span class="tick"><Icon name="check" /></span>
          <div>
            <h3>{result.files.length > 1 ? t.doneMany : t.doneOne}</h3>
            <div class="sub">
              {#if result.text !== undefined}{t.reviewText}{:else}{result.files[0].name} · {humanSize(result.files[0].blob.size)}{/if}
            </div>
          </div>
        </div>

        {#if result.text !== undefined}
          <div class="text-result">
            <div class="tr-head">
              <span class="meta">{t.extractedText.replace('{n}', result.text.length.toLocaleString())}</span>
              <button class="btn btn-ghost btn-sm" onclick={copyText}><Icon name="copy" /> {t.copy}</button>
            </div>
            <pre tabindex="0" aria-label="Extracted text" data-testid="result-text">{result.text}</pre>
          </div>
          <div class="download-row">
            {#each result.files as f (f.name)}
              <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(f.blob, f.name)}>
                <Icon name="download" /> {f.name} <span class="size">({humanSize(f.blob.size)})</span>
              </button>
            {/each}
          </div>
        {:else}
          <div class="download-row">
            {#each result.files as f (f.name)}
              <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(f.blob, f.name)}>
                <Icon name="download" /> {f.name} <span class="size">({humanSize(f.blob.size)})</span>
              </button>
            {/each}
          </div>
        {/if}

        <div class="made-here"><Icon name="shield" /> {t.madeHere}</div>
      </div>

      <div class="action-row"><button class="btn btn-ghost" onclick={reset}>{t.startOver}</button></div>
    {/if}
  </div>
{/if}
