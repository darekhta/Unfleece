<script lang="ts" module>
  export interface NewSignature {
    bytes: Uint8Array;
    imageType: 'png' | 'jpg';
    aspect: number;
    save: boolean;
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte';

  let { open = $bindable(false), oncreate }: { open?: boolean; oncreate: (sig: NewSignature) => void } = $props();

  type Tab = 'draw' | 'type' | 'upload';
  let tab = $state<Tab>('draw');
  let save = $state(true);
  let err = $state('');

  // Draw tab
  let pad: HTMLCanvasElement | undefined;
  let drawing = false;
  let padSized = false;
  let hasInk = $state(false);
  // Type tab
  let typed = $state('');
  // Upload tab
  let fileInput: HTMLInputElement | undefined;
  let dialogEl: HTMLDivElement | undefined;

  const CURSIVE = '"Snell Roundhand", "Savoye LET", "Segoe Script", "Brush Script MT", cursive';

  $effect(() => {
    if (open) {
      err = '';
      requestAnimationFrame(() => dialogEl?.focus());
    } else {
      tab = 'draw';
      typed = '';
      hasInk = false;
      padSized = false;
      drawing = false;
    }
  });

  function close() { open = false; }
  function onKey(e: KeyboardEvent) { if (open && e.key === 'Escape') close(); }

  // ---- draw pad (backing sized to the on-screen box → uniform mapping) ----
  function ctx() { const c = pad?.getContext('2d'); if (!c) throw new Error('no canvas'); return c; }
  function pos(e: PointerEvent) {
    const r = pad!.getBoundingClientRect();
    return { x: (e.clientX - r.left) * (pad!.width / r.width), y: (e.clientY - r.top) * (pad!.height / r.height) };
  }
  function sizePad() {
    if (padSized || !pad) return;
    const r = pad.getBoundingClientRect();
    if (r.width < 10 || r.height < 10) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    pad.width = Math.round(r.width * dpr);
    pad.height = Math.round(r.height * dpr);
    padSized = true;
  }
  function down(e: PointerEvent) {
    sizePad();
    drawing = true;
    const c = ctx(); const p = pos(e);
    const r = pad!.getBoundingClientRect();
    c.strokeStyle = '#0b1220'; c.lineWidth = 5.5 * (pad!.width / r.width); c.lineCap = 'round'; c.lineJoin = 'round';
    c.beginPath(); c.moveTo(p.x, p.y);
    pad!.setPointerCapture(e.pointerId);
  }
  function moveDraw(e: PointerEvent) {
    if (!drawing) return;
    const c = ctx(); const p = pos(e); c.lineTo(p.x, p.y); c.stroke(); hasInk = true;
  }
  function endDraw() { drawing = false; }
  function clearPad() { if (!pad) return; ctx().clearRect(0, 0, pad.width, pad.height); hasInk = false; err = ''; }

  /** Crop a canvas to its ink bounding box (keeps the drawn proportions). */
  function cropInk(c: HTMLCanvasElement): HTMLCanvasElement | null {
    const g = c.getContext('2d');
    if (!g) return null;
    const { width: W, height: H } = c;
    const data = g.getImageData(0, 0, W, H).data;
    let minX = W, minY = H, maxX = -1, maxY = -1;
    for (let y = 0; y < H; y++) {
      const row = y * W * 4;
      for (let x = 0; x < W; x++) {
        if (data[row + x * 4 + 3] > 8) {
          if (x < minX) minX = x;
          if (x > maxX) maxX = x;
          if (y < minY) minY = y;
          if (y > maxY) maxY = y;
        }
      }
    }
    if (maxX < 0) return null;
    const padPx = Math.round(Math.min(W, H) * 0.05) + 2;
    minX = Math.max(0, minX - padPx); minY = Math.max(0, minY - padPx);
    maxX = Math.min(W - 1, maxX + padPx); maxY = Math.min(H - 1, maxY + padPx);
    const off = document.createElement('canvas');
    off.width = maxX - minX + 1;
    off.height = maxY - minY + 1;
    off.getContext('2d')!.drawImage(c, minX, minY, off.width, off.height, 0, 0, off.width, off.height);
    return off;
  }

  /** Render typed text in a cursive script to a canvas. */
  function typeToCanvas(text: string): HTMLCanvasElement {
    const font = `64px ${CURSIVE}`;
    const probe = document.createElement('canvas').getContext('2d')!;
    probe.font = font;
    const m = probe.measureText(text);
    const asc = m.actualBoundingBoxAscent || 48;
    const desc = m.actualBoundingBoxDescent || 16;
    const padPx = 18;
    const c = document.createElement('canvas');
    c.width = Math.max(2, Math.ceil(m.width + padPx * 2));
    c.height = Math.max(2, Math.ceil(asc + desc + padPx * 2));
    const g = c.getContext('2d')!;
    g.font = font;
    g.fillStyle = '#0b1220';
    g.fillText(text, padPx, padPx + asc);
    return c;
  }

  async function emit(c: HTMLCanvasElement) {
    const blob = await new Promise<Blob>((res, rej) =>
      c.toBlob((b) => (b ? res(b) : rej(new Error('Could not capture signature'))), 'image/png'),
    );
    oncreate({ bytes: new Uint8Array(await blob.arrayBuffer()), imageType: 'png', aspect: c.height / c.width, save });
    close();
  }
  async function useDrawn() {
    if (!pad) return;
    const cropped = cropInk(pad);
    if (!cropped) { err = 'Draw your signature first.'; return; }
    await emit(cropped);
  }
  async function useTyped() {
    const t = typed.trim();
    if (!t) { err = 'Type your name first.'; return; }
    await emit(typeToCanvas(t));
  }
  async function onUpload(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const imageType = file.type === 'image/png' ? 'png' : file.type === 'image/jpeg' ? 'jpg' : null;
    if (!imageType) { err = 'Choose a PNG or JPG image.'; input.value = ''; return; }
    try {
      const url = URL.createObjectURL(file);
      const img = new Image();
      await new Promise<void>((res, rej) => {
        img.onload = () => res();
        img.onerror = () => rej(new Error('Could not read that image'));
        img.src = url;
      });
      URL.revokeObjectURL(url);
      const aspect = img.naturalWidth ? img.naturalHeight / img.naturalWidth : 0.32;
      oncreate({ bytes: new Uint8Array(await file.arrayBuffer()), imageType, aspect, save });
      close();
    } catch (e2) {
      err = e2 instanceof Error ? e2.message : String(e2);
    } finally {
      input.value = '';
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="sigmodal-backdrop" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) close(); }}>
    <div class="sigmodal" role="dialog" aria-modal="true" aria-label="Create signature" tabindex="-1" bind:this={dialogEl} data-testid="signature-modal">
      <div class="sm-head">
        <h3>Create your signature</h3>
        <button class="btn-icon sm-close" type="button" onclick={close} aria-label="Close"><Icon name="x" /></button>
      </div>

      <div class="sm-tabs" role="tablist" aria-label="Signature method">
        <button type="button" role="tab" class={tab === 'draw' ? 'on' : ''} aria-selected={tab === 'draw'} onclick={() => { tab = 'draw'; err = ''; }} data-testid="sig-tab-draw">Draw</button>
        <button type="button" role="tab" class={tab === 'type' ? 'on' : ''} aria-selected={tab === 'type'} onclick={() => { tab = 'type'; err = ''; }} data-testid="sig-tab-type">Type</button>
        <button type="button" role="tab" class={tab === 'upload' ? 'on' : ''} aria-selected={tab === 'upload'} onclick={() => { tab = 'upload'; err = ''; }} data-testid="sig-tab-upload">Upload</button>
      </div>

      {#if tab === 'draw'}
        <div class="sm-padwrap">
          <canvas
            bind:this={pad}
            class="sm-pad"
            role="img"
            aria-label="Signature drawing area"
            data-testid="sign-canvas"
            onpointerdown={down} onpointermove={moveDraw} onpointerup={endDraw} onpointercancel={endDraw}
          ></canvas>
          <button class="btn btn-ghost btn-sm sm-clear" type="button" onclick={clearPad}><Icon name="x" /> Clear</button>
        </div>
      {:else if tab === 'type'}
        <input class="field" placeholder="Type your name" bind:value={typed} aria-label="Type your signature" data-testid="sig-type-input" />
        <div class="sm-preview" style={`font-family:${CURSIVE}`}>{typed || 'Your name'}</div>
      {:else}
        <button class="sm-upload" type="button" onclick={() => fileInput?.click()}>
          <Icon name="upload" /> Choose a PNG or JPG…
        </button>
        <input bind:this={fileInput} type="file" accept="image/png,image/jpeg" style="display:none" data-testid="signature-file-input" onchange={onUpload} />
      {/if}

      {#if err}<p class="sm-err" role="alert">{err}</p>{/if}

      <label class="checkbox-row sm-save">
        <input type="checkbox" bind:checked={save} />
        <span class="checkbox-box"><Icon name="check" /></span>
        Save in this browser for reuse
      </label>

      <div class="sm-actions">
        <button class="btn btn-ghost" type="button" onclick={close}>Cancel</button>
        {#if tab === 'draw'}
          <button class="btn btn-primary" type="button" disabled={!hasInk} onclick={useDrawn} data-testid="sig-use">Use signature</button>
        {:else if tab === 'type'}
          <button class="btn btn-primary" type="button" disabled={!typed.trim()} onclick={useTyped} data-testid="sig-use">Use signature</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .sigmodal-backdrop {
    position: fixed; inset: 0; z-index: 100;
    background: rgba(7, 9, 15, 0.55);
    display: grid; place-items: center; padding: 1rem;
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }
  .sigmodal {
    width: 100%; max-width: 560px;
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: 1.25rem;
    display: flex; flex-direction: column; gap: 0.9rem;
    outline: none;
  }
  /* bottom sheet on phones */
  @media (max-width: 640px) {
    .sigmodal-backdrop { place-items: end center; padding: 0; }
    .sigmodal { max-width: 100%; border-radius: var(--radius-lg) var(--radius-lg) 0 0; padding-bottom: max(1.25rem, env(safe-area-inset-bottom)); }
  }
  .sm-head { display: flex; justify-content: space-between; align-items: center; }
  .sm-head h3 { font-size: 1.05rem; }
  .sm-tabs { display: flex; gap: 0.25rem; background: var(--surface-sunken); padding: 0.25rem; border-radius: var(--radius-sm); }
  .sm-tabs button {
    flex: 1; min-height: 40px; border: none; background: none;
    border-radius: calc(var(--radius-sm) - 2px); color: var(--muted);
    font-weight: 600; cursor: pointer; font-size: 0.9rem;
  }
  .sm-tabs button.on { background: var(--surface-1); color: var(--ink); box-shadow: var(--shadow-sm); }
  .sm-padwrap { position: relative; }
  .sm-pad {
    width: 100%; height: 220px; display: block;
    background: var(--surface-inverse);
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    cursor: crosshair; touch-action: none;
  }
  .sm-clear { position: absolute; right: 0.5rem; bottom: 0.5rem; }
  .sm-preview {
    min-height: 96px; display: grid; place-items: center;
    font-size: 2.4rem; color: #0b1220; overflow: hidden;
    background: var(--surface-inverse);
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 0.5rem 1rem; text-align: center; word-break: break-word;
  }
  .sm-upload {
    min-height: 110px; width: 100%;
    display: flex; align-items: center; justify-content: center; gap: 0.5rem;
    border: 1.5px dashed var(--border-strong); border-radius: var(--radius-sm);
    background: var(--surface-sunken); color: var(--ink-soft);
    font-weight: 600; cursor: pointer; font-size: 0.95rem;
  }
  .sm-upload:hover { border-color: var(--border-accent); }
  .sm-err { color: var(--error); font-size: 0.875rem; }
  .sm-save { font-size: 0.9rem; color: var(--muted); }
  .sm-actions { display: flex; justify-content: flex-end; gap: 0.6rem; flex-wrap: wrap; }
</style>
