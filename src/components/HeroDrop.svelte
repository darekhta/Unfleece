<script lang="ts">
  import Icon from './Icon.svelte';
  import { stashFiles } from '../lib/handoff.js';

  type Action = { name: string; icon: string; href: string };
  const PDF_ACTIONS: Action[] = [
    { name: 'Merge PDF', icon: 'merge', href: '/tools/merge-pdf' },
    { name: 'Split PDF', icon: 'split', href: '/tools/split-pdf' },
    { name: 'Compress', icon: 'compress', href: '/tools/compress-pdf' },
    { name: 'Images → PDF', icon: 'imgToPdf', href: '/tools/images-to-pdf' },
  ];
  const IMG_ACTIONS: Action[] = [
    { name: 'Images → PDF', icon: 'imgToPdf', href: '/tools/images-to-pdf' },
    { name: 'Convert image', icon: 'convertImg', href: '/tools/convert-image' },
  ];

  let files = $state<File[]>([]);
  let dragOver = $state(false);
  let busy = $state(false);
  let inputEl: HTMLInputElement | undefined;

  const isImage = $derived(files.length > 0 && files.every((f) => f.type.startsWith('image/')));
  const actions = $derived(files.length === 0 ? PDF_ACTIONS : isImage ? IMG_ACTIONS : PDF_ACTIONS);

  function add(list: FileList | null) {
    if (!list || !list.length) return;
    files = Array.from(list);
  }
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    add(e.dataTransfer?.files ?? null);
  }
  async function go(href: string) {
    busy = true;
    try {
      if (files.length) await stashFiles(files);
    } catch (e) {
      /* fall through — tool page will just show an empty dropzone */
    }
    window.location.assign(href);
  }
</script>

<div class="hero-proof glass">
  <button
    class="dropzone {dragOver ? 'dragover' : ''}"
    type="button"
    onclick={() => inputEl?.click()}
    ondragover={(e) => { e.preventDefault(); dragOver = true; }}
    ondragleave={() => (dragOver = false)}
    ondrop={onDrop}
    data-testid="hero-dropzone"
    aria-label="Drop a PDF or image to get started — processed on your device, no file upload."
  >
    <span class="dz-icon"><Icon name={files.length ? 'check' : 'upload'} /></span>
    <span class="dz-primary">{files.length ? `${files[0].name} ready` : 'Drop a PDF to start'}</span>
    <span class="dz-types">{files.length ? 'Now pick what to do ↓' : 'PDF · JPG · PNG'}</span>
  </button>
  <input
    bind:this={inputEl}
    type="file"
    accept="application/pdf,image/png,image/jpeg"
    multiple
    style="display:none"
    onchange={(e) => add((e.currentTarget as HTMLInputElement).files)}
    data-testid="hero-file-input"
  />

  <div>
    <div class="popular">{files.length ? 'Do this with it' : 'Popular'}</div>
    <div class="hero-chips">
      {#each actions as a (a.href + a.name)}
        {#if files.length}
          <button class="hero-chip" type="button" disabled={busy} onclick={() => go(a.href)}>
            <Icon name={a.icon} /> {a.name}
          </button>
        {:else}
          <a class="hero-chip" href={a.href}><Icon name={a.icon} /> {a.name}</a>
        {/if}
      {/each}
    </div>
  </div>
</div>
