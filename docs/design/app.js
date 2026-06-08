/* ============================================================
   UNFLEECE — App logic: rendering, views, runner state machine
   ============================================================ */
(function () {
  'use strict';

  /* ---------- Helpers ---------- */
  function el(html) { const t = document.createElement('template'); t.innerHTML = html.trim(); return t.content.firstElementChild; }
  function ic(name, sw) { return '<span class="i" data-icon="' + name + '"' + (sw ? ' data-sw="' + sw + '"' : '') + '></span>'; }

  /* ============================================================
     1. BENTO GRID
     ============================================================ */
  function toolCard(t) {
    const c = catMeta(t.cat);
    const chipCls = c.cat ? 'chip ' + c.cat : 'chip';
    let cls = 'tool-card';
    if (t.feature) cls += ' feature';
    if (t.wide) cls += ' wide';
    const card = el(
      '<a class="' + cls + '" href="#" data-tool="' + t.slug + '" style="--card-glow:color-mix(in srgb,' + c.hue + ' 22%, transparent)" aria-label="' + t.name + ' \u2014 ' + t.tag + '">' +
        '<div class="tc-head">' +
          '<span class="' + chipCls + '" data-icon="' + t.icon + '"></span>' +
          (t.wasm ? '<span class="badge-wasm">Rust \u2192 WASM</span>' : '<span class="open-hint">Open ' + ic('arrowRight', 2) + '</span>') +
        '</div>' +
        '<div class="tc-body">' +
          '<div class="tc-name">' + t.name + '</div>' +
          '<div class="tc-tag">' + t.tag + '</div>' +
          (t.feature && t.desc ? '<div class="tc-desc">' + t.desc + '</div>' : '') +
        '</div>' +
        (t.wide ? '<span class="open-hint" style="margin-left:auto">Open ' + ic('arrowRight', 2) + '</span>' : '') +
      '</a>'
    );
    return card;
  }

  function renderBento() {
    const host = document.getElementById('bento-host');
    if (!host) return;
    CATEGORIES.forEach(function (c) {
      const tools = toolsByCat(c.id);
      const band = el('<section class="cat-band" id="cat-' + c.id + '" aria-labelledby="h-' + c.id + '"></section>');
      band.appendChild(el(
        '<div class="cat-divider">' +
          '<span class="eyebrow cat-eyebrow" id="h-' + c.id + '"><span class="swatch" style="background:' + c.hue + '"></span>' + c.label + '</span>' +
          '<span class="count">' + tools.length + (tools.length === 1 ? ' tool' : ' tools') + '</span>' +
          '<span class="descriptor">' + c.descriptor + '</span>' +
        '</div>'
      ));
      const grid = el('<div class="bento"></div>');
      tools.forEach(function (t) { grid.appendChild(toolCard(t)); });
      band.appendChild(grid);
      host.appendChild(band);
    });
  }

  function renderRelated() {
    const host = document.getElementById('related-host');
    if (!host) return;
    ['split-pdf', 'extract-pages', 'remove-pages', 'reorder'].forEach(function (slug) {
      const t = TOOLS.find(function (x) { return x.slug === slug; });
      const c = catMeta(t.cat);
      host.appendChild(el(
        '<a class="related-card" href="#" data-tool="' + t.slug + '">' +
          '<span class="chip ' + (c.cat || '') + '" data-icon="' + t.icon + '"></span>' +
          '<span class="rc-name">' + t.name + '</span>' +
        '</a>'
      ));
    });
  }

  /* ============================================================
     2. TOOL RUNNER — state machine
     ============================================================ */
  const SAMPLE_FILES = [
    { name: 'report-q1.pdf', size: '1.2 MB' },
    { name: 'appendix.pdf', size: '340 KB' },
  ];
  let runnerState = 'idle';
  let files = [];

  const STATES = [
    { id: 'idle', label: 'Idle' },
    { id: 'files', label: 'Files added' },
    { id: 'options', label: 'Options form' },
    { id: 'working', label: 'Working' },
    { id: 'progress', label: 'Working (per-page)' },
    { id: 'done', label: 'Done \u00b7 single' },
    { id: 'zip', label: 'Done \u00b7 zip' },
    { id: 'text', label: 'Done \u00b7 text' },
    { id: 'error', label: 'Error' },
    { id: 'largefile', label: 'Large-file warning' },
    { id: 'sign', label: 'Sign canvas' },
    { id: 'fillform', label: 'Fill form' },
    { id: 'nofields', label: 'No fields found' },
  ];

  /* ---- reusable fragments ---- */
  function dropzone(compact, multi) {
    return '<button class="dropzone' + (compact ? ' compact' : '') + '" id="dropzone" type="button" ' +
      'aria-label="Add PDFs \u2014 drag and drop, or activate to choose files. Files are processed on your device.">' +
      '<span class="dz-icon" data-icon="upload"></span>' +
      '<span class="dz-primary">Drop PDF' + (multi ? 's' : '') + ' here, or click to choose</span>' +
      '<span class="dz-types">PDF only \u00b7 ' + (multi ? 'single or multiple' : 'one file') + '</span>' +
      '<span class="dz-privacy">' + ic('lock') + ' Files stay on your device \u2014 no file upload.</span>' +
      '</button>';
  }
  function fileList(arr) {
    if (!arr.length) return '';
    let rows = arr.map(function (f) {
      return '<li class="file-row' + (f.status === 'done' ? ' done' : '') + '">' +
        '<span class="grip" aria-hidden="true">' + ic('grip') + '</span>' +
        '<span class="f-icon" data-icon="pdfToText"></span>' +
        '<span class="f-main"><span class="f-name" title="' + f.name + '">' + f.name + '</span>' +
        '<span class="f-size">' + f.size + '</span></span>' +
        '<span class="f-status">' +
          (f.status === 'done' ? '<span class="check-ok" aria-label="Done">' + ic('check') + '</span>' :
           f.status === 'working' ? '<span class="spinner" style="color:var(--accent-400)"></span>' : '') +
          '<button class="f-remove" type="button" aria-label="Remove ' + f.name + '">' + ic('x') + '</button>' +
        '</span></li>';
    }).join('');
    return '<div><div class="file-meta-label">Added files</div><ul class="file-list">' + rows + '</ul></div>';
  }
  function optionsForm() {
    return '<form class="options-panel" aria-label="Tool options" onsubmit="return false">' +
      '<h3>Options</h3>' +
      '<div class="options-grid">' +
        '<div class="span-2">' +
          '<label class="label" for="opt-pages">Pages <span class="req" aria-hidden="true">*</span></label>' +
          '<input class="field" id="opt-pages" value="1-3, 5, 8-10" inputmode="numeric" aria-describedby="opt-pages-help">' +
          '<div class="range-preview"><span class="range-chip">1</span><span class="range-chip">2</span><span class="range-chip">3</span><span class="range-chip">5</span><span class="range-chip">8</span><span class="range-chip">9</span><span class="range-chip">10</span><span class="range-summary">\u00b7 7 pages</span></div>' +
          '<div class="help" id="opt-pages-help">Numbers, commas and dashes \u2014 e.g. 1-3, 5, 8-10.</div>' +
        '</div>' +
        '<div><label class="label" for="opt-size">Page size</label>' +
          '<select class="field" id="opt-size"><option>Match source</option><option>A4</option><option>US Letter</option><option>Fit to content</option></select></div>' +
        '<div><label class="label" for="opt-margin">Margin (pt)</label>' +
          '<input class="field" id="opt-margin" type="number" value="24" min="0" max="144" step="4"></div>' +
        '<div class="span-2"><label class="label" for="opt-wm">Header text</label>' +
          '<input class="field" id="opt-wm" placeholder="e.g. Confidential \u2014 \u041d\u0435 \u043f\u0435\u0440\u0435\u0441\u0438\u043b\u0430\u0442\u0438"></div>' +
        '<div class="span-2"><label class="checkbox-row"><input type="checkbox" checked>' +
          '<span class="checkbox-box">' + ic('check') + '</span><span>Add a page-number footer</span></label></div>' +
      '</div></form>';
  }
  function actionRow(label, opts) {
    opts = opts || {};
    let run;
    if (opts.working) {
      run = '<button class="btn btn-primary" disabled aria-busy="true"><span class="spinner"></span>' + (opts.verb || 'Working\u2026') + '</button>';
    } else {
      run = '<button class="btn btn-primary" id="run-btn"' + (opts.disabled ? ' disabled' : '') + '>' + label + ' ' + ic('arrowRight', 2) + '</button>';
    }
    return '<div class="action-row">' + run +
      (opts.startover ? '<button class="btn btn-ghost" id="startover">Start over</button>' : '') +
      (opts.disabled ? '<span class="action-hint">Add a file first.</span>' : '') +
      '</div>';
  }
  function steps(active) {
    const arr = [['Add file', 1], ['Set options', 2], ['Run', 3], ['Download', 4]];
    return '<div class="runner-steps" aria-hidden="true">' + arr.map(function (s, i) {
      const on = i <= active ? ' on' : '';
      return '<span class="s' + on + '"><span class="num">' + s[1] + '</span>' + s[0] + '</span>' +
        (i < arr.length - 1 ? '<span class="arrow">' + ic('chevronRight') + '</span>' : '');
    }).join('') + '</div>';
  }
  function privacyEcho() {
    return '<div class="made-here">' + ic('shield') + ' Made right here in your browser. We never saw it.</div>';
  }

  /* ---- per-state body ---- */
  function bodyFor(state) {
    switch (state) {
      case 'idle':
        return steps(0) + '<div class="runner-body">' + dropzone(false, true) +
          actionRow('Merge PDFs', { disabled: true }) + '</div>';

      case 'files':
        return steps(2) + '<div class="runner-body">' + dropzone(true, true) +
          fileList(SAMPLE_FILES) + actionRow('Merge PDFs', { startover: false }) + '</div>';

      case 'options':
        return steps(1) + '<div class="runner-body">' + dropzone(true, true) +
          fileList(SAMPLE_FILES) + optionsForm() + actionRow('Apply', {}) + '</div>';

      case 'working':
        return steps(2) + '<div class="runner-body">' + dropzone(true, true) +
          fileList(SAMPLE_FILES.map(function (f) { return Object.assign({}, f, { status: 'working' }); })) +
          '<div class="progress-block"><div class="progress-label"><span class="spinner" style="color:var(--accent-400)"></span> Working on your file\u2026</div>' +
          '<div class="progress-track indeterminate"><span class="progress-fill"></span></div></div>' +
          actionRow('Merge PDFs', { working: true, verb: 'Working\u2026' }) + '</div>';

      case 'progress':
        return steps(2) + '<div class="runner-body">' + dropzone(true, true) +
          fileList(SAMPLE_FILES) +
          '<div class="progress-block"><div class="progress-label">Rendering page 7 of 20\u2026</div>' +
          '<div class="progress-track" role="progressbar" aria-valuemin="0" aria-valuemax="20" aria-valuenow="7"><span class="progress-fill" style="width:35%"></span></div></div>' +
          actionRow('Rendering', { working: true, verb: 'Rendering\u2026' }) + '</div>';

      case 'done':
        return steps(3) + '<div class="runner-body">' +
          '<div class="result-panel" id="result-panel" tabindex="-1">' +
            '<div class="result-head"><span class="tick">' + ic('check') + '</span><div><h3>Done \u2014 your file is ready.</h3><div class="sub">merged.pdf \u00b7 1.5 MB</div></div></div>' +
            '<div class="download-row"><button class="btn btn-primary download-btn">' + ic('download') + ' merged.pdf <span class="size">(1.5 MB)</span></button></div>' +
            privacyEcho() +
          '</div>' + actionRow('', { startover: true, hideRun: true }).replace(/<button class="btn btn-primary"[^>]*>.*?<\/button>/, '') + '</div>';

      case 'zip':
        return steps(3) + '<div class="runner-body">' +
          '<div class="result-panel" tabindex="-1">' +
            '<div class="result-head"><span class="tick">' + ic('check') + '</span><div><h3>Done \u2014 12 pages, packed into a .zip.</h3><div class="sub">report-jpg.zip \u00b7 4.1 MB</div></div></div>' +
            '<div class="download-row"><button class="btn btn-primary download-btn">' + ic('download') + ' Download 12 images (ZIP) <span class="size">(4.1 MB)</span></button></div>' +
            privacyEcho() +
          '</div>' +
          '<div class="action-row"><button class="btn btn-ghost" id="startover">Start over</button></div></div>';

      case 'text':
        return steps(3) + '<div class="runner-body">' +
          '<div class="result-panel" tabindex="-1">' +
            '<div class="result-head"><span class="tick">' + ic('check') + '</span><div><h3>Done \u2014 here\u2019s your text.</h3><div class="sub">Copy it or download the .txt.</div></div></div>' +
            '<div class="text-result"><div class="tr-head"><span class="meta">Extracted text \u00b7 1,284 characters</span>' +
              '<span style="display:flex;gap:.5rem"><button class="btn btn-ghost btn-sm">' + ic('copy') + ' Copy</button><button class="btn btn-ghost btn-sm">' + ic('download') + ' .txt</button></span></div>' +
              '<pre tabindex="0" aria-label="Extracted text">QUARTERLY REPORT \u2014 Q1 2026\n\nExecutive summary\nRevenue grew 18% quarter over quarter, driven by\nnew privacy-first product lines. Operating margin\nheld steady at 24%.\n\n\u0420\u0435\u0437\u044e\u043c\u0435: \u0432\u0438\u0440\u0443\u0447\u043a\u0430 \u0437\u0440\u043e\u0441\u043b\u0430 \u043d\u0430 18% \u0437\u0430 \u043a\u0432\u0430\u0440\u0442\u0430\u043b.\n\nKey metrics\n  \u2022 MAU ............ 1.2M\n  \u2022 Retention ...... 94%\n  \u2022 NPS ............ 71</pre></div>' +
            privacyEcho() +
          '</div>' +
          '<div class="action-row"><button class="btn btn-ghost" id="startover">Start over</button></div></div>';

      case 'error':
        return steps(2) + '<div class="runner-body">' + dropzone(true, true) + fileList(SAMPLE_FILES) +
          '<div class="alert alert-error" role="alert"><span class="ico" data-icon="alert"></span>' +
          '<div class="a-body"><strong>We couldn\u2019t open this PDF.</strong><p>It may be damaged or password-protected. No file left your device \u2014 unlock it in your reader and try again.</p></div></div>' +
          '<div class="action-row"><button class="btn btn-ghost" id="startover">Start over</button></div></div>';

      case 'largefile':
        return steps(2) + '<div class="runner-body">' + dropzone(true, true) +
          fileList([{ name: 'scan-archive.pdf', size: '182 MB' }]) +
          '<div class="alert alert-warn" role="status"><span class="ico" data-icon="alert"></span>' +
          '<div class="a-body"><strong>Large file (182 MB).</strong><p>Everything runs in your browser, so very large files may be slow or hit your browser\u2019s memory limit. We\u2019ll try anyway.</p></div></div>' +
          actionRow('Merge PDFs', {}) + '</div>';

      case 'sign':
        return steps(1) + '<div class="runner-body">' + dropzone(true, false) + fileList([{ name: 'contract.pdf', size: '210 KB' }]) +
          '<div class="options-panel"><div class="sign-pad-wrap">' +
            '<div class="sign-pad-head"><span class="label">Draw your signature</span>' +
              '<button class="btn btn-ghost btn-sm" id="sign-clear">' + ic('x') + ' Clear</button></div>' +
            '<canvas class="sign-canvas" id="sign-canvas" role="img" aria-label="Signature drawing area. Draw with mouse, touch or stylus."></canvas>' +
            '<div class="sign-hint">This adds a visual signature mark \u2014 it isn\u2019t a cryptographic e-signature. ' +
              '<a class="btn-link" href="#" onclick="return false">Upload a signature image instead</a></div>' +
          '</div></div>' +
          '<div class="options-panel"><h3>Signature placement</h3><div class="options-grid" style="grid-template-columns:repeat(4,1fr)">' +
            '<div><label class="label" for="s-page">Page</label><input class="field" id="s-page" type="number" value="1" min="1"></div>' +
            '<div><label class="label" for="s-x">X (pt)</label><input class="field" id="s-x" type="number" value="360"></div>' +
            '<div><label class="label" for="s-y">Y (pt)</label><input class="field" id="s-y" type="number" value="90"></div>' +
            '<div><label class="label" for="s-w">Width (pt)</label><input class="field" id="s-w" type="number" value="160"></div>' +
          '</div></div>' +
          actionRow('Place signature', {}) + '</div>';

      case 'fillform':
        return steps(1) + '<div class="runner-body">' + dropzone(true, false) + fileList([{ name: 'application-form.pdf', size: '88 KB' }]) +
          '<div class="options-panel"><h3>Detected fields \u00b7 5 found</h3><div class="options-grid">' +
            '<div><label class="label" for="ff-1">Full name (text)</label><input class="field" id="ff-1" value="\u041e\u043b\u0435\u043a\u0441\u0430\u043d\u0434\u0440 \u041a\u043e\u0432\u0430\u043b\u0435\u043d\u043a\u043e"></div>' +
            '<div><label class="label" for="ff-2">Email (text)</label><input class="field" id="ff-2" value="hello@example.com"></div>' +
            '<div><label class="label" for="ff-3">Country (dropdown)</label><select class="field" id="ff-3"><option>Ukraine</option><option>Poland</option><option>Germany</option></select></div>' +
            '<div><label class="label" for="ff-4">Date (text)</label><input class="field" id="ff-4" value="2026-06-06"></div>' +
            '<div class="span-2"><label class="checkbox-row"><input type="checkbox" checked><span class="checkbox-box">' + ic('check') + '</span><span>I agree to the terms (checkbox)</span></label></div>' +
          '</div></div>' +
          actionRow('Fill form', {}) + '</div>';

      case 'nofields':
        return steps(1) + '<div class="runner-body">' + dropzone(true, false) + fileList([{ name: 'flat-scan.pdf', size: '1.4 MB' }]) +
          '<div class="empty-state"><span class="sheep" data-icon="sheep" data-sw="1.5"></span>' +
            '<h3>No fillable fields here.</h3><p>We didn\u2019t find any AcroForm fields \u2014 this might be a flat scan, or it uses XFA forms, which aren\u2019t supported. You could <a class="btn-link" href="#" onclick="return false">Sign</a> it instead.</p></div>' +
          '<div class="action-row"><button class="btn btn-ghost" id="startover">Start over</button></div></div>';
    }
    return '';
  }

  function renderRunner() {
    const host = document.getElementById('runner-body-host');
    if (!host) return;
    host.innerHTML = bodyFor(runnerState);
    hydrateIcons(host);
    wireRunner(host);
    var fab = document.getElementById('rf-select');
    if (fab) fab.value = runnerState;
    if (runnerState === 'sign') initSignCanvas();
  }

  function setState(s) { runnerState = s; renderRunner(); }

  function wireRunner(host) {
    const dz = host.querySelector('#dropzone');
    if (dz) dz.addEventListener('click', function () {
      if (runnerState === 'idle') setState('files');
    });
    const run = host.querySelector('#run-btn');
    if (run && !run.disabled) run.addEventListener('click', function () {
      setState('working');
      setTimeout(function () { if (runnerState === 'working') setState('done'); }, 1600);
    });
    const so = host.querySelector('#startover');
    if (so) so.addEventListener('click', function () { setState('idle'); });
    host.querySelectorAll('.f-remove').forEach(function (b) {
      b.addEventListener('click', function () {
        const row = b.closest('.file-row'); if (row) row.remove();
      });
    });
    // move focus to result on done states
    const rp = host.querySelector('#result-panel, .result-panel');
    if (rp && rp.hasAttribute('tabindex')) { try { rp.focus({ preventScroll: true }); } catch (e) {} }
  }

  function renderStateChips() {
    // Discreet floating reviewer control — a compact dock, not a page billboard.
    if (document.getElementById('runner-fab')) return;
    var opts = STATES.map(function (s) { return '<option value="' + s.id + '">' + s.label + '</option>'; }).join('');
    var fab = el(
      '<div class="runner-fab" id="runner-fab" hidden>' +
        '<span class="rf-tag">Demo</span>' +
        '<label class="sr-only" for="rf-select">Runner state</label>' +
        '<select class="rf-select" id="rf-select" aria-label="Runner state">' + opts + '</select>' +
      '</div>'
    );
    document.body.appendChild(fab);
    fab.querySelector('#rf-select').addEventListener('change', function (e) { setState(e.target.value); });
  }

  /* ============================================================
     3. SIGN CANVAS
     ============================================================ */
  function initSignCanvas() {
    const canvas = document.getElementById('sign-canvas');
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = Math.round(rect.width * dpr);
    canvas.height = Math.round(rect.height * dpr);
    const ctx = canvas.getContext('2d');
    ctx.scale(dpr, dpr);
    ctx.strokeStyle = '#0b1220';
    ctx.lineWidth = 2.4; ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    let drawing = false, last = null;
    function pos(e) { const r = canvas.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; }
    canvas.addEventListener('pointerdown', function (e) { drawing = true; last = pos(e); canvas.setPointerCapture(e.pointerId); });
    canvas.addEventListener('pointermove', function (e) {
      if (!drawing) return; const p = pos(e);
      ctx.beginPath(); ctx.moveTo(last.x, last.y);
      const mx = (last.x + p.x) / 2, my = (last.y + p.y) / 2;
      ctx.quadraticCurveTo(last.x, last.y, mx, my); ctx.stroke();
      last = p;
    });
    canvas.addEventListener('pointerup', function () { drawing = false; });
    canvas.addEventListener('pointerleave', function () { drawing = false; });
    // pre-seed a friendly signature squiggle so the state reads well in review
    ctx.beginPath();
    ctx.moveTo(rect.width * 0.16, rect.height * 0.66);
    ctx.bezierCurveTo(rect.width * 0.28, rect.height * 0.2, rect.width * 0.38, rect.height * 0.9, rect.width * 0.48, rect.height * 0.5);
    ctx.bezierCurveTo(rect.width * 0.56, rect.height * 0.2, rect.width * 0.62, rect.height * 0.85, rect.width * 0.78, rect.height * 0.46);
    ctx.stroke();
    const clear = document.getElementById('sign-clear');
    if (clear) clear.addEventListener('click', function () { ctx.clearRect(0, 0, canvas.width, canvas.height); });
  }

  /* ============================================================
     4. VIEW SWITCHING + THEME
     ============================================================ */
  function showView(name) {
    document.querySelectorAll('.view').forEach(function (v) { v.classList.toggle('active', v.dataset.view === name); });
    document.querySelectorAll('.demo-dock [data-view-link]').forEach(function (b) {
      b.setAttribute('aria-current', b.dataset.viewLink === name ? 'page' : 'false');
    });
    var fab = document.getElementById('runner-fab');
    if (fab) fab.hidden = (name !== 'tool');
    window.scrollTo({ top: 0, behavior: 'auto' });
  }

  function initTheme() {
    const saved = localStorage.getItem('uf-theme2');
    if (saved) document.documentElement.setAttribute('data-theme', saved);
    const btn = document.getElementById('theme-toggle');
    if (btn) btn.addEventListener('click', function () {
      const cur = document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
      const next = cur === 'light' ? 'dark' : 'light';
      document.documentElement.setAttribute('data-theme', next);
      localStorage.setItem('uf-theme2', next);
      btn.setAttribute('aria-label', 'Switch to ' + (next === 'light' ? 'dark' : 'light') + ' theme');
    });
  }

  /* ============================================================
     5. INIT
     ============================================================ */
  function init() {
    renderBento();
    renderRelated();
    renderStateChips();
    renderRunner();
    hydrateIcons(document);
    initTheme();

    // any [data-view-link] (CTAs, tool cards, brand, demo dock)
    document.addEventListener('click', function (e) {
      const nav = e.target.closest('[data-nav]');
      if (nav) {
        e.preventDefault();
        showView('home');
        const map = { tools: 'tools-section', how: 'how-section', privacy: 'how-section' };
        const t = document.getElementById(map[nav.dataset.nav]);
        if (t) requestAnimationFrame(function () { window.scrollTo({ top: t.getBoundingClientRect().top + window.scrollY - 72, behavior: 'smooth' }); });
        return;
      }
      const link = e.target.closest('[data-view-link]');
      if (link) { e.preventDefault(); showView(link.dataset.viewLink); return; }
      const tool = e.target.closest('[data-tool]');
      if (tool) { e.preventDefault(); showView('tool'); }
    });
    // scroll-anchor CTA to tools
    const browse = document.getElementById('browse-cta');
    if (browse) browse.addEventListener('click', function (e) {
      e.preventDefault(); showView('home');
      const t = document.getElementById('tools-section');
      if (t) requestAnimationFrame(function () { window.scrollTo({ top: t.getBoundingClientRect().top + window.scrollY - 72, behavior: 'smooth' }); });
    });

    showView('home');
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
