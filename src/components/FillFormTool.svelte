<script lang="ts">
  import type { Tool } from '../lib/registry.js';
  import type { FormField } from '../lib/tools/forms.js';
  import type { FriendlyError } from '../lib/errors.js';
  import Icon from './Icon.svelte';
  import Dropzone from './Dropzone.svelte';
  import ProgressBlock from './ProgressBlock.svelte';
  import { downloadBlob, humanSize, outputBaseName } from '../lib/download.js';
  import { onMount } from 'svelte';
  import { takePendingFiles } from '../lib/handoff.js';
  import { friendlyError } from '../lib/errors.js';
  import { reportError, engineForTool } from '../lib/telemetry.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../lib/focus.js';

  let { tool }: { tool: Tool } = $props();

  let files = $state<File[]>([]);
  let fields = $state<FormField[]>([]);
  let values = $state<Record<string, string | boolean>>({});
  let status = $state<'idle' | 'loading' | 'ready' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('filled.pdf');
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;
  let loadVersion = 0;

  function initialFieldValue(field: FormField): string | boolean {
    if (field.type === 'checkbox') return Boolean(field.value);
    if (typeof field.value === 'string') return field.value;
    return field.options?.[0] ?? '';
  }

  onMount(async () => {
    try {
      const pending = await takePendingFiles();
      if (pending && pending.length) files = [pending[0]];
    } catch (e) {
      /* ignore */
    }
  });

  $effect(() => {
    const file = files[0];
    const version = ++loadVersion;
    if (!file) {
      fields = [];
      values = {};
      status = 'idle';
      error = null;
      resultBlob = null;
      return;
    }
    status = 'loading';
    error = null;
    resultBlob = null;
    (async () => {
      try {
        const { getPdfWorker } = await import('../lib/worker/client.js');
        const bytes = new Uint8Array(await file.arrayBuffer());
        const detected = await getPdfWorker().listFormFields(bytes);
        if (version !== loadVersion) return;
        fields = detected.filter((f) => f.type === 'text' || f.type === 'checkbox' || f.type === 'dropdown' || f.type === 'radio' || f.type === 'optionlist');
        values = Object.fromEntries(fields.map((field) => [field.name, initialFieldValue(field)]));
        status = 'ready';
      } catch (e) {
        if (version !== loadVersion) return;
        reportError(tool.id, e, engineForTool(tool.id));
        error = friendlyError(e, "Couldn't read that form");
        status = 'error';
        await focusAfterUpdate(() => errorEl);
      }
    })();
  });

  async function fill() {
    status = 'working';
    error = null;
    resultBlob = null;
    try {
      const { getPdfWorker } = await import('../lib/worker/client.js');
      const bytes = new Uint8Array(await files[0].arrayBuffer());
      const out = await getPdfWorker().fillForm(bytes, values);
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = outputBaseName(files[0].name) + '-filled.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      reportError(tool.id, e, engineForTool(tool.id));
      error = friendlyError(e, "Couldn't fill that form");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    }
  }
  function reset() {
    loadVersion++;
    files = [];
    fields = [];
    values = {};
    status = 'idle';
    error = null;
    resultBlob = null;
    void focusDropzoneAfterUpdate();
  }
</script>

<div class="runner-body">
  {#if status !== 'done'}
    <Dropzone accept={tool.accept} multiple={false} bind:files />

    {#if status === 'loading'}
      <ProgressBlock fallback="Detecting form fields…" />
    {/if}

    {#if (status === 'ready' || status === 'working') && fields.length > 0}
      <div class="options-panel">
        <h3>Detected fields · {fields.length} found</h3>
        <div class="options-grid">
          {#each fields as field (field.name)}
            {#if field.type === 'checkbox'}
              <div class="span-2">
                <label class="checkbox-row">
                  <input type="checkbox" bind:checked={values[field.name]} />
                  <span class="checkbox-box"><Icon name="check" /></span>
                  <span>{field.name} <span class="muted">(checkbox)</span></span>
                </label>
              </div>
            {:else if field.options && field.options.length > 0}
              <div>
                <label class="label" for={`f-${field.name}`}>{field.name} <span class="muted">({field.type})</span></label>
                <select id={`f-${field.name}`} class="field" bind:value={values[field.name]}>
                  {#each field.options as option (option)}
                    <option value={option}>{option}</option>
                  {/each}
                </select>
              </div>
            {:else}
              <div>
                <label class="label" for={`f-${field.name}`}>{field.name} <span class="muted">({field.type})</span></label>
                <input id={`f-${field.name}`} class="field" type="text" bind:value={values[field.name]} />
              </div>
            {/if}
          {/each}
        </div>
      </div>
      {#if status === 'working'}
        <ProgressBlock fallback="Filling form fields…" />
      {/if}
      <div class="action-row">
        {#if status === 'working'}
          <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Filling…</button>
        {:else}
          <button class="btn btn-primary" data-testid="run-button" onclick={fill}>Fill form <Icon name="arrowRight" sw={2} /></button>
        {/if}
      </div>
    {/if}

    {#if status === 'ready' && fields.length === 0}
      <div class="empty-state">
        <span class="sheep"><Icon name="sheep" sw={1.5} /></span>
        <h3>No fillable fields here.</h3>
        <p>We didn't find any AcroForm fields — this might be a flat scan, or it uses XFA forms, which aren't supported.</p>
      </div>
    {/if}

    {#if status === 'error' && error}
      <div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error">
        <span class="ico"><Icon name="alert" /></span>
        <div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div>
      </div>
    {/if}
  {/if}

  {#if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head">
        <span class="tick"><Icon name="check" /></span>
        <div><h3>Filled — your file is ready.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div>
      </div>
      <div class="download-row">
        <button class="btn btn-primary download-btn" data-testid="download-button" onclick={() => downloadBlob(resultBlob!, resultName)}>
          <Icon name="download" /> {resultName} <span class="size">({humanSize(resultBlob.size)})</span>
        </button>
      </div>
      <div class="made-here"><Icon name="shield" /> Made right here in your browser. We never saw it.</div>
    </div>
    <div class="action-row"><button class="btn btn-ghost" onclick={reset}>Start over</button></div>
  {/if}
</div>
