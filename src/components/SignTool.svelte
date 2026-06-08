<script lang="ts">
  import type { Tool } from '../lib/registry.js';
  import type { FriendlyError } from '../lib/errors.js';
  import type { RunProgress } from '../lib/progress.js';
  import { onDestroy, onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import Dropzone from './Dropzone.svelte';
  import ProgressBlock from './ProgressBlock.svelte';
  import PdfStage, { type StageInfo } from './editor/PdfStage.svelte';
  import Handle from './editor/Handle.svelte';
  import SignatureModal, { type NewSignature } from './SignatureModal.svelte';
  import { startDrag } from '../lib/browser/useDrag.js';
  import { downloadBlob, humanSize } from '../lib/download.js';
  import { takePendingFiles } from '../lib/handoff.js';
  import { friendlyError } from '../lib/errors.js';
  import { focusAfterUpdate, focusDropzoneAfterUpdate } from '../lib/focus.js';
  import { listSignatures, saveSignature, deleteSignature } from '../lib/signatures.js';
  import { snapRectToGuides, type SnapGuides } from '../lib/browser/stageCoords.js';

  let { tool }: { tool: Tool } = $props();

  let files = $state<File[]>([]);
  let bytes = $state<Uint8Array | null>(null);
  let stageInfo = $state<StageInfo>();
  let modalOpen = $state(false);
  let ready = $state(false);

  // ---- signature objects: independent of any document ----
  interface Sig {
    id: string;
    bytes: Uint8Array;
    imageType: 'png' | 'jpg';
    aspect: number;
    url: string; // object URL owned here
    savedId?: string; // present when persisted in the local library
    srcBlob?: Blob; // library blob — direct byte source (no fetch(blob:) fragility)
  }
  let sigs = $state<Sig[]>([]);
  let sigStatus = $state('');
  /** The signature a page-tap will place: last created/last tapped, else the newest. */
  let activeSigId = $state<string | null>(null);
  function currentSig(): Sig | undefined {
    return sigs.find((s) => s.id === activeSigId) ?? sigs[0];
  }

  // ---- placements: one stamp per entry, each on its own page/position ----
  interface Placement {
    id: string;
    page: number;
    /** fractions of the page */
    x: number;
    y: number;
    w: number;
    aspect: number;
    url: string; // per-placement object URL (owned + revoked here)
    image: { bytes: Uint8Array; imageType: 'png' | 'jpg' };
    pageWpt: number;
    pageHpt: number;
  }
  let placements = $state<Placement[]>([]);
  let activeGuides = $state<(SnapGuides & { page: number }) | null>(null);

  let status = $state<'idle' | 'working' | 'done' | 'error'>('idle');
  let error = $state<FriendlyError | null>(null);
  let resultBlob = $state<Blob | null>(null);
  let resultName = $state('signed.pdf');
  let progress = $state<RunProgress | null>(null);
  let errorEl: HTMLDivElement | null = null;
  let resultEl: HTMLDivElement | null = null;

  const uid = () =>
    typeof crypto !== 'undefined' && 'randomUUID' in crypto ? crypto.randomUUID() : `p-${Math.floor(performance.now() * 1000)}`;

  onMount(async () => {
    try { const p = await takePendingFiles(); if (p && p.length) files = [p[0]]; } catch (e) { /* ignore */ }
    try {
      const list = await listSignatures();
      sigs = list.map((s) => ({
        id: s.id,
        bytes: new Uint8Array(0), // hydrated lazily on first placement
        imageType: s.imageType === 'jpg' ? 'jpg' as const : 'png' as const,
        aspect: Number.isFinite(s.aspect) && s.aspect > 0 ? s.aspect : 0.32,
        url: URL.createObjectURL(s.blob),
        savedId: s.id,
        srcBlob: s.blob,
      }));
      // hydrate bytes in the background so placing is instant
      for (const s of list) {
        s.blob.arrayBuffer().then((buf) => {
          sigs = sigs.map((cur) => (cur.id === s.id ? { ...cur, bytes: new Uint8Array(buf) } : cur));
        });
      }
    } catch (e) { /* library unavailable — drawing still works */ }
    ready = true;
  });
  onDestroy(() => {
    for (const s of sigs) URL.revokeObjectURL(s.url);
    for (const p of placements) URL.revokeObjectURL(p.url);
  });
  $effect(() => {
    const f = files[0];
    if (!f) { bytes = null; return; }
    f.arrayBuffer().then((b) => (bytes = new Uint8Array(b)));
  });

  // ---- create / manage signatures ----
  async function onCreate(n: NewSignature) {
    const sig: Sig = {
      id: uid(),
      bytes: n.bytes,
      imageType: n.imageType,
      aspect: n.aspect,
      url: URL.createObjectURL(new Blob([n.bytes as BlobPart], { type: n.imageType === 'png' ? 'image/png' : 'image/jpeg' })),
    };
    sigs = [sig, ...sigs];
    activeSigId = sig.id;
    sigStatus = 'Signature created.';
    if (n.save) {
      try {
        const saved = await saveSignature(new Blob([n.bytes as BlobPart], { type: n.imageType === 'png' ? 'image/png' : 'image/jpeg' }), n.imageType, n.aspect);
        sigs = sigs.map((s) => (s.id === sig.id ? { ...s, savedId: saved.id } : s));
        sigStatus = 'Signature created and saved in this browser.';
      } catch (e) {
        // storage unavailable (private mode, quota, …) — keep it for this session and say so
        sigStatus = 'Signature created for this session — saving in this browser failed.';
      }
    }
    if (bytes) await place(sig);
  }
  async function removeSig(sig: Sig) {
    if (sig.savedId) {
      try { await deleteSignature(sig.savedId); } catch (e) { /* ignore */ }
    }
    URL.revokeObjectURL(sig.url);
    sigs = sigs.filter((s) => s.id !== sig.id);
    sigStatus = 'Signature deleted.';
  }

  // ---- placements ----
  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi);

  /** The stage publishes its dims only after pdf.js finishes rendering — wait for it. */
  async function stageReady(): Promise<StageInfo | undefined> {
    for (let i = 0; i < 200; i++) {
      const s = stageInfo;
      if (s && s.pageWidthPt) return s;
      if (!bytes) return undefined;
      await new Promise((r) => setTimeout(r, 50));
    }
    return undefined;
  }

  async function place(sig: Sig, at?: { fx: number; fy: number }) {
    try {
      const s = await stageReady();
      if (!s || !bytes) return;
      let image = { bytes: sig.bytes, imageType: sig.imageType };
      if (image.bytes.length === 0 && sig.srcBlob) {
        // not hydrated yet — read the library blob directly
        image = { ...image, bytes: new Uint8Array(await sig.srcBlob.arrayBuffer()) };
      }
      if (image.bytes.length === 0) {
        throw new Error('This saved signature could not be read on this device. Delete it (×) and create it again — it will save correctly now.');
      }
      // Start small and unobtrusive: ~18% of the page width, capped so tall
      // images never eat more than ~16% of the page height.
      let w = 0.18;
      const hFrac = (frac: number) => (frac * s.cssWidth * sig.aspect) / s.cssHeight;
      if (hFrac(w) > 0.16) w = (0.16 * s.cssHeight) / (sig.aspect * s.cssWidth);
      w = clamp(w, 0.05, 0.5);
      const n = placements.filter((p) => p.page === s.pageIndex).length;
      const off = at ? 0 : Math.min(n * 0.05, 0.3);
      const cx = at ? at.fx : 0.5;
      const cy = at ? at.fy : 0.55;
      const url = URL.createObjectURL(new Blob([image.bytes as BlobPart], { type: image.imageType === 'png' ? 'image/png' : 'image/jpeg' }));
      placements = [
        ...placements,
        {
          id: uid(),
          page: s.pageIndex,
          x: clamp(cx - w / 2 + off, 0, 1 - w),
          y: clamp(cy - hFrac(w) / 2 + off, 0, Math.max(0, 1 - hFrac(w))),
          w,
          aspect: sig.aspect,
          url,
          image,
          pageWpt: s.pageWidthPt,
          pageHpt: s.pageHeightPt,
        },
      ];
      if (status === 'error') { error = null; status = 'idle'; }
      sigStatus = `Signature placed on page ${s.pageIndex + 1}.`;
    } catch (e) {
      // a tap must never silently do nothing — surface exactly what went wrong
      error = friendlyError(e, "Couldn't place the signature");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    }
  }
  function movePlacement(id: string, dxF: number, dyF: number, s: StageInfo) {
    placements = placements.map((p) => {
      if (p.id !== id) return p;
      const hFrac = (p.w * s.cssWidth * p.aspect) / s.cssHeight;
      const raw = {
        x: (p.x + dxF) * s.cssWidth,
        y: (p.y + dyF) * s.cssHeight,
        w: p.w * s.cssWidth,
        h: hFrac * s.cssHeight,
      };
      const snapped = snapRectToGuides(raw, s);
      activeGuides = Object.keys(snapped.guides).length > 0 ? { page: s.pageIndex, ...snapped.guides } : null;
      return {
        ...p,
        x: clamp(snapped.rect.x / s.cssWidth, 0, 1 - p.w),
        y: clamp(snapped.rect.y / s.cssHeight, 0, Math.max(0, 1 - hFrac)),
      };
    });
  }
  function resizePlacement(id: string, dxF: number) {
    placements = placements.map((p) => (p.id === id ? { ...p, w: clamp(p.w + dxF, 0.05, 1 - p.x) } : p));
  }
  function removePlacement(id: string) {
    const p = placements.find((q) => q.id === id);
    if (p) URL.revokeObjectURL(p.url);
    placements = placements.filter((q) => q.id !== id);
  }
  function clearPlacements() {
    for (const p of placements) URL.revokeObjectURL(p.url);
    placements = [];
  }
  function placementDown(e: PointerEvent, id: string, s: StageInfo) {
    startDrag(e, {
      onStart: () => { activeGuides = null; },
      onMove: (dx, dy) => movePlacement(id, dx / s.cssWidth, dy / s.cssHeight, s),
      onEnd: () => { activeGuides = null; },
    });
  }
  const pagesSigned = $derived([...new Set(placements.map((p) => p.page + 1))].sort((a, b) => a - b));

  async function apply() {
    if (!bytes) return;
    if (placements.length === 0) {
      error = { title: 'Nothing placed yet', message: 'Tap a signature to place it on the page first.' };
      status = 'error';
      await focusAfterUpdate(() => errorEl);
      return;
    }
    status = 'working';
    error = null;
    resultBlob = null;
    progress = { phase: 'working', label: 'Stamping signatures locally…' };
    try {
      const stamps = placements.map((p) => {
        const width = p.w * p.pageWpt;
        const height = width * p.aspect;
        return {
          image: p.image.bytes,
          imageType: p.image.imageType,
          pageIndex: p.page,
          x: p.x * p.pageWpt,
          y: p.pageHpt - p.y * p.pageHpt - height,
          width,
          height,
        };
      });
      const { stampImageMany } = await import('../lib/tools/sign.js');
      const out = await stampImageMany(bytes, stamps);
      resultBlob = new Blob([out as BlobPart], { type: 'application/pdf' });
      resultName = files[0].name.replace(/\.[^.]+$/, '') + '-signed.pdf';
      status = 'done';
      await focusAfterUpdate(() => resultEl);
    } catch (e) {
      error = friendlyError(e, "Couldn't sign");
      status = 'error';
      await focusAfterUpdate(() => errorEl);
    } finally {
      progress = null;
    }
  }
  function reset() {
    files = [];
    bytes = null;
    status = 'idle';
    resultBlob = null;
    error = null;
    progress = null;
    clearPlacements();
    // signatures intentionally survive — the library is independent of the document
    void focusDropzoneAfterUpdate();
  }
</script>

{#snippet sigLibrary(canPlace: boolean)}
  {#if sigs.length > 0}
    <div class="sig-grid" data-testid="saved-signatures">
      {#each sigs as g (g.id)}
        <span class="saved-sig">
          <button
            type="button"
            class="saved-pick"
            onclick={() => { activeSigId = g.id; if (canPlace) void place(g); }}
            aria-label={canPlace ? 'Place this signature on the page' : 'Signature'}
            data-testid="sig-thumb"
          >
            <img src={g.url} alt="Signature" />
          </button>
          <button type="button" class="saved-del" onclick={() => void removeSig(g)} aria-label="Delete signature"><Icon name="x" /></button>
        </span>
      {/each}
    </div>
    <p class="help">
      {canPlace ? 'Tap a signature to place it on this page. Drag to position, corner dot to resize.' : 'Saved only in this browser — never uploaded.'}
    </p>
  {:else}
    <p class="help">No signatures yet. Create one — it stays only in this browser.</p>
  {/if}
  <button class="btn btn-ghost" type="button" onclick={() => (modalOpen = true)} data-testid="create-signature">
    <Icon name="sign" /> New signature
  </button>
{/snippet}

<div class="runner-body" data-testid="sign-tool" data-ready={ready ? 'true' : 'false'}>
  {#if !bytes}
    <Dropzone accept={tool.accept} multiple={false} bind:files />
    <div class="options-panel sig-manager">
      <h3>Your signatures</h3>
      {@render sigLibrary(false)}
    </div>
  {:else if status === 'done' && resultBlob}
    <div class="result-panel" tabindex="-1" role="status" aria-live="polite" bind:this={resultEl} data-testid="result">
      <div class="result-head"><span class="tick"><Icon name="check" /></span><div><h3>Signed{pagesSigned.length > 1 ? ` — ${placements.length} signatures on pages ${pagesSigned.join(', ')}` : ''}.</h3><div class="sub">{resultName} · {humanSize(resultBlob.size)}</div></div></div>
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
            {@const onPage = placements.filter((p) => p.page === s.pageIndex)}
            <!-- Tap anywhere on the page to drop the signature right there
                 (the DocuSign-style interaction people expect). -->
            <div
              class="place-catcher"
              role="button"
              tabindex="-1"
              aria-label="Place your signature here"
              data-testid="place-catcher"
              onclick={(e) => {
                const sig = currentSig();
                if (!sig) { modalOpen = true; return; }
                const v = s.clientToView(e.clientX, e.clientY);
                void place(sig, { fx: v.x / s.cssWidth, fy: v.y / s.cssHeight });
              }}
            ></div>
            {#if activeGuides?.page === s.pageIndex}
              {#if activeGuides.vertical !== undefined}
                <div class="snap-guide snap-guide-v" style={`left:${activeGuides.vertical}px`} data-testid="snap-guide-vertical"></div>
              {/if}
              {#if activeGuides.horizontal !== undefined}
                <div class="snap-guide snap-guide-h" style={`top:${activeGuides.horizontal}px`} data-testid="snap-guide-horizontal"></div>
              {/if}
            {/if}
            {#each onPage as p (p.id)}
              {@const wPx = p.w * s.cssWidth}
              {@const hPx = wPx * p.aspect}
              {@const lx = p.x * s.cssWidth}
              {@const ty = p.y * s.cssHeight}
              <div
                class="stamp"
                style={`left:${lx}px;top:${ty}px;width:${wPx}px;height:${hPx}px`}
                onpointerdown={(e) => placementDown(e, p.id, s)}
                role="application"
                aria-label="Signature — drag to move"
                data-testid="placement"
              >
                <img src={p.url} alt="" draggable="false" />
                <button
                  type="button"
                  class="stamp-remove"
                  aria-label="Remove this signature"
                  onpointerdown={(e) => e.stopPropagation()}
                  onclick={() => removePlacement(p.id)}
                ><Icon name="x" /></button>
                <Handle variant="corner" label="Resize signature" style={`left:${wPx}px;top:${hPx}px;transform:translate(-50%,-50%)`} onmove={(dx) => resizePlacement(p.id, dx / s.cssWidth)} />
              </div>
            {/each}
            {#if onPage.length === 0}
              <div class="stamp-hint">{sigs.length ? 'Tap anywhere on the page to place your signature' : 'Tap the page to create your signature'}</div>
            {/if}
          {/snippet}
        </PdfStage>
      {/key}

      <!-- Mobile: compact sticky action bar — the document keeps the screen. -->
      <div class="mobile-bar">
        {#if status === 'error' && error}
          <div class="mb-error" role="alert">{error.title} — {error.message}</div>
        {/if}
        <div class="mb-row">
          <div class="mb-sigs" aria-label="Your signatures">
            {#each sigs as g (g.id)}
              <button
                type="button"
                class="mb-thumb {currentSig()?.id === g.id ? 'on' : ''}"
                onclick={() => { activeSigId = g.id; void place(g); }}
                aria-label="Place this signature on the page"
              ><img src={g.url} alt="" /></button>
            {/each}
            <button type="button" class="mb-new" onclick={() => (modalOpen = true)} aria-label="New signature">+</button>
          </div>
          <button class="btn-icon mb-reset" type="button" onclick={reset} aria-label="Choose another file"><Icon name="upload" /></button>
          {#if status === 'working'}
            <button class="btn btn-primary mb-sign" disabled aria-busy="true"><span class="spinner"></span></button>
          {:else}
            <button class="btn btn-primary mb-sign" disabled={placements.length === 0} onclick={apply}>
              Sign{placements.length > 0 ? ` (${placements.length})` : ''}
            </button>
          {/if}
        </div>
      </div>

      <div class="options-panel editor-controls">
        <h3>Your signatures</h3>
        {@render sigLibrary(true)}

        <div class="sr-only" role="status" aria-live="polite" aria-atomic="true">{sigStatus}</div>

        {#if placements.length > 0}
          <p class="placed-line">
            {placements.length} placed{pagesSigned.length > 1 ? ` on pages ${pagesSigned.join(', ')}` : ''}
            <button class="btn-link" type="button" onclick={clearPlacements}>clear all</button>
          </p>
        {/if}

        {#if status === 'error' && error}<div class="alert alert-error" role="alert" tabindex="-1" bind:this={errorEl} data-testid="error"><span class="ico"><Icon name="alert" /></span><div class="a-body"><strong>{error.title}</strong><p>{error.message}</p></div></div>{/if}

        {#if status === 'working'}
          <ProgressBlock {progress} fallback="Stamping signatures locally…" />
        {/if}

        <div class="action-row" style="margin-top:1rem">
          {#if status === 'working'}
            <button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span> Signing…</button>
          {:else}
            <button class="btn btn-primary" data-testid="run-button" disabled={placements.length === 0} onclick={apply}>
              {placements.length > 1 ? `Sign ${placements.length} places` : 'Sign PDF'} <Icon name="arrowRight" sw={2} />
            </button>
          {/if}
          <button class="btn btn-ghost" onclick={reset}>Choose another file</button>
        </div>
        <p class="sign-hint">This is a visual signature mark, not a cryptographic e-signature.</p>
      </div>
    </div>
  {/if}
</div>

<SignatureModal bind:open={modalOpen} oncreate={(n) => void onCreate(n)} />

<style>
  .editor-layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 1.5rem; align-items: start; }
  /* on phones, signatures first (controls above the page) */
  .mobile-bar { display: none; }
  @media (max-width: 820px) {
    /* phones: the document gets the screen; controls collapse into a pinned thumb bar */
    .editor-layout { grid-template-columns: minmax(0, 1fr); padding-bottom: 84px; }
    .editor-controls { display: none; }
    .mobile-bar {
      display: flex; flex-direction: column; gap: 0.4rem;
      /* fixed, not sticky: the runner shell clips overflow, which kills sticky */
      position: fixed; left: 0.75rem; right: 0.75rem;
      bottom: max(0.75rem, env(safe-area-inset-bottom)); z-index: 60;
      padding: 0.5rem;
      background: var(--surface-glass-strong);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      box-shadow: var(--shadow-lg);
      backdrop-filter: blur(14px) saturate(1.15);
      -webkit-backdrop-filter: blur(14px) saturate(1.15);
    }
    .mb-row { display: flex; align-items: center; gap: 0.5rem; }
    .mb-sigs { display: flex; align-items: center; gap: 0.4rem; overflow-x: auto; flex: 1; min-width: 0; padding: 2px; -webkit-overflow-scrolling: touch; }
    .mb-thumb {
      flex: none; width: 64px; height: 40px; padding: 2px;
      background: var(--surface-inverse); border: 1.5px solid var(--border); border-radius: var(--radius-xs); cursor: pointer;
    }
    .mb-thumb.on { border-color: var(--accent-500); }
    .mb-thumb img { width: 100%; height: 100%; object-fit: contain; }
    .mb-new {
      flex: none; width: 44px; height: 40px;
      border: 1.5px dashed var(--border-strong); border-radius: var(--radius-xs);
      background: var(--surface-sunken); color: var(--muted);
      font-size: 1.3rem; font-weight: 600; line-height: 1; cursor: pointer;
    }
    .mb-reset { flex: none; width: 44px; height: 44px; }
    .mb-sign { flex: none; min-width: 92px; }
    .mb-error { font-size: 0.8125rem; color: var(--error); }
  }
  .stamp { position: absolute; cursor: move; touch-action: none; z-index: 2; outline: 1.5px dashed var(--accent-500); }
  .stamp img { width: 100%; height: 100%; object-fit: contain; pointer-events: none; user-select: none; }
  .stamp-remove {
    position: absolute; top: -12px; right: -12px;
    width: 24px; height: 24px; padding: 0;
    display: grid; place-items: center;
    border-radius: 50%; border: 1.5px solid #fff;
    background: var(--error); color: #fff; cursor: pointer;
    box-shadow: var(--shadow-md); z-index: 4; touch-action: none;
  }
  .stamp-remove :global(svg) { width: 12px; height: 12px; }
  .place-catcher { position: absolute; inset: 0; cursor: copy; }
  .snap-guide {
    position: absolute;
    pointer-events: none;
    z-index: 1;
    background: var(--accent-500);
    box-shadow: 0 0 0 1px rgba(255,255,255,0.65);
    opacity: 0.85;
  }
  .snap-guide-v { top: 0; bottom: 0; width: 1.5px; transform: translateX(-0.75px); }
  .snap-guide-h { left: 0; right: 0; height: 1.5px; transform: translateY(-0.75px); }
  .stamp-hint { position: absolute; left: 12px; top: 12px; background: rgba(46,204,143,0.14); border: 1px solid var(--border-accent); color: #0b1220; font-size: 0.8rem; padding: 0.3rem 0.6rem; border-radius: var(--radius-sm); pointer-events: none; }
  .sig-manager { margin-top: 0.25rem; }
  .sig-manager h3 { margin-bottom: 0.6rem; }
  .sig-grid { display: flex; flex-wrap: wrap; gap: 0.6rem; margin-bottom: 0.5rem; }
  .saved-sig { position: relative; display: inline-flex; }
  .saved-pick {
    width: 96px; height: 48px; padding: 3px; cursor: pointer;
    background: var(--surface-inverse); border: 1px solid var(--border); border-radius: var(--radius-xs);
    transition: border-color var(--dur-fast) var(--ease-out), transform var(--dur-fast) var(--ease-out);
  }
  .saved-pick:hover { border-color: var(--border-accent); transform: translateY(-1px); }
  .saved-pick img { width: 100%; height: 100%; object-fit: contain; }
  .saved-del {
    position: absolute; top: -8px; right: -8px;
    width: 20px; height: 20px; padding: 0; display: grid; place-items: center;
    border-radius: 50%; border: 1.5px solid #fff;
    background: var(--muted); color: #fff; cursor: pointer; box-shadow: var(--shadow-sm);
  }
  .saved-del:hover { background: var(--error); }
  .saved-del :global(svg) { width: 10px; height: 10px; }
  .placed-line { font-size: 0.875rem; color: var(--muted); margin-top: 0.75rem; display: flex; align-items: center; gap: 0.6rem; flex-wrap: wrap; }
  .sign-hint { font-size: 0.8125rem; color: var(--muted-dim); margin-top: 0.75rem; }
</style>
