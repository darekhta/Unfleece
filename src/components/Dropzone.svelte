<script lang="ts">
  import Icon from './Icon.svelte';
  import { humanSize } from '../lib/download.js';
  import { fileMatchesAccept } from '../lib/handoff.js';
  import { onMount } from 'svelte';

  let {
    accept,
    multiple = false,
    files = $bindable([]),
  }: { accept: string; multiple?: boolean; files?: File[] } = $props();

  let dragOver = $state(false);
  let rejected = $state('');
  let liveMessage = $state('');
  let hydrated = $state(false);
  let inputEl: HTMLInputElement | undefined;

  const kind = $derived(accept.includes('pdf') ? 'PDF' : accept.includes('image') ? 'Image' : 'File');

  function addFiles(list: FileList | null) {
    if (!list || list.length === 0) return;
    const arr = Array.from(list);
    const accepted = arr.filter((f) => fileMatchesAccept(f, accept));
    const rejectedCount = arr.length - accepted.length;
    if (rejectedCount > 0) {
      rejected = rejectedCount === 1
        ? `One file does not match this tool.`
        : `${rejectedCount} files do not match this tool.`;
    } else if (!multiple && accepted.length > 1) {
      rejected = 'This tool uses one file at a time, so only the first matching file was added.';
    } else {
      rejected = '';
    }
    if (accepted.length === 0) return;
    const addedFiles = multiple ? accepted : accepted.slice(0, 1);
    files = multiple ? [...files, ...addedFiles] : addedFiles;
    const added = addedFiles.length === 1
      ? `${addedFiles[0].name} added.`
      : `${addedFiles.length} files added.`;
    liveMessage = `${added} ${files.length} ${files.length === 1 ? 'file' : 'files'} ready.`;
  }
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    addFiles(e.dataTransfer?.files ?? null);
  }
  function remove(i: number) {
    const removed = files[i];
    files = files.filter((_, idx) => idx !== i);
    liveMessage = `${removed?.name ?? 'File'} removed. ${files.length} ${files.length === 1 ? 'file' : 'files'} remaining.`;
  }
  function move(i: number, delta: -1 | 1) {
    const nextIndex = i + delta;
    if (nextIndex < 0 || nextIndex >= files.length) return;
    const next = [...files];
    const [file] = next.splice(i, 1);
    next.splice(nextIndex, 0, file);
    files = next;
    liveMessage = `${file.name} moved to position ${nextIndex + 1} of ${files.length}.`;
  }

  // ---- drag-to-reorder (pointer events → mouse + touch; buttons above stay for keyboards) ----
  let dragIndex = $state<number | null>(null);
  let dragOffset = $state(0);
  let dragStartY = 0;
  let dragFrom = 0;
  let dragSnapshot: File[] = [];
  let rowH = 0;

  function gripDown(e: PointerEvent, i: number) {
    if (files.length < 2) return;
    e.preventDefault();
    const row = (e.currentTarget as HTMLElement).closest('.file-row') as HTMLElement | null;
    rowH = (row?.offsetHeight ?? 52) + 8; // row + list gap
    dragStartY = e.clientY;
    dragFrom = i;
    dragIndex = i;
    dragSnapshot = [...files];
    dragOffset = 0;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function gripMove(e: PointerEvent) {
    if (dragIndex === null) return;
    const dy = e.clientY - dragStartY;
    const target = Math.min(Math.max(dragFrom + Math.round(dy / rowH), 0), dragSnapshot.length - 1);
    if (target !== dragIndex) {
      const next = [...dragSnapshot];
      const [f] = next.splice(dragFrom, 1);
      next.splice(target, 0, f);
      files = next;
      dragIndex = target;
    }
    // keep the row under the pointer relative to its current slot
    dragOffset = dy - (dragIndex - dragFrom) * rowH;
  }
  function gripUp() {
    if (dragIndex === null) return;
    const f = files[dragIndex];
    liveMessage = `${f.name} moved to position ${dragIndex + 1} of ${files.length}.`;
    dragIndex = null;
    dragOffset = 0;
  }

  onMount(() => {
    hydrated = true;
  });
</script>

<div data-testid="dropzone-shell" data-ready={hydrated ? 'true' : 'false'}>
  <div class="sr-only" role="status" aria-live="polite" aria-atomic="true">{liveMessage}</div>

  <button
    class="dropzone {files.length ? 'compact' : ''} {dragOver ? 'dragover' : ''}"
    type="button"
    onclick={() => inputEl?.click()}
    ondragover={(e) => { e.preventDefault(); dragOver = true; }}
    ondragleave={() => (dragOver = false)}
    ondrop={onDrop}
    data-testid="dropzone"
    aria-label="Add files — drag and drop, or activate to choose files. Files are processed on your device."
  >
    <span class="dz-icon"><Icon name="upload" /></span>
    <span class="dz-primary">Drop {multiple ? `${kind}s` : `a ${kind}`} here, or click to choose</span>
    <span class="dz-types">{kind} · {multiple ? 'single or multiple' : 'one file'}</span>
    <span class="dz-privacy"><Icon name="lock" /> Files stay on your device — nothing is uploaded.</span>
  </button>

  <input
    bind:this={inputEl}
    type="file"
    {accept}
    {multiple}
    style="display:none"
    onchange={(e) => addFiles((e.currentTarget as HTMLInputElement).files)}
    data-testid="file-input"
  />

  {#if files.length > 0}
    <div style="margin-top:1rem">
      <div class="file-meta-label">Added files</div>
      <ul class="file-list">
        {#each files as file, i (file)}
          <li
            class="file-row {dragIndex === i ? 'dragging' : ''}"
            style={dragIndex === i ? `transform: translateY(${dragOffset}px)` : ''}
          >
            {#if multiple && files.length > 1}
              <button
                class="grip"
                type="button"
                aria-label={`Drag to reorder ${file.name} (or use the arrow buttons)`}
                data-testid="file-grip"
                onpointerdown={(e) => gripDown(e, i)}
                onpointermove={gripMove}
                onpointerup={gripUp}
                onpointercancel={gripUp}
              ><Icon name="grip" /></button>
            {/if}
            <span class="f-icon"><Icon name="pdfToText" /></span>
            <span class="f-main">
              <span class="f-name" title={file.name}>{file.name}</span>
              <span class="f-size">{humanSize(file.size)}</span>
            </span>
            <span class="f-status">
              {#if multiple && files.length > 1}
                <button class="f-move move-up" type="button" aria-label={`Move ${file.name} up`} disabled={i === 0} onclick={() => move(i, -1)}>
                  <Icon name="chevronRight" />
                </button>
                <button class="f-move move-down" type="button" aria-label={`Move ${file.name} down`} disabled={i === files.length - 1} onclick={() => move(i, 1)}>
                  <Icon name="chevronRight" />
                </button>
              {/if}
              <button class="f-remove" type="button" aria-label={`Remove ${file.name}`} onclick={() => remove(i)}>
                <Icon name="x" />
              </button>
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if rejected}
    <div class="field-error" role="status">
      <Icon name="alert" />
      {rejected}
    </div>
  {/if}
</div>
