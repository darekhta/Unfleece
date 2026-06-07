import { test, expect, type Page } from '@playwright/test';
import { samplePdfBuffer, protectedPdfBuffer, PNG_1x1, pdfFile, pngFile, tiffFile, textFile, tiff1x1Buffer } from './_helpers';

async function setFiles(page: Page, files: { name: string; mimeType: string; buffer: Buffer }[]) {
  await expect(page.getByTestId('dropzone-shell')).toHaveAttribute('data-ready', 'true');
  await page.getByTestId('file-input').setInputFiles(files);
  await expect
    .poll(async () => {
      const fileRows = await page.locator('.file-row').count();
      const dropzones = await page.getByTestId('dropzone-shell').count();
      return fileRows > 0 || dropzones === 0;
    })
    .toBe(true);
}

async function runAndDownload(page: Page, timeout = 30000): Promise<string> {
  await expect(page.getByTestId('run-button')).toBeEnabled();
  await page.getByTestId('run-button').click();
  const result = page.getByTestId('result');
  await expect(result).toBeVisible({ timeout });
  await expect(result).toBeFocused();
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByTestId('download-button').first().click(),
  ]);
  return download.suggestedFilename();
}

// Simulate a real native file drag-and-drop (Playwright has no first-class file DnD).
async function dropFile(page: Page, testid: string, name: string, buffer: Buffer, type: string) {
  const dt = await page.evaluateHandle(
    ({ data, name, type }) => {
      const t = new DataTransfer();
      t.items.add(new File([new Uint8Array(data)], name, { type }));
      return t;
    },
    { data: Array.from(buffer), name, type },
  );
  const el = page.getByTestId(testid);
  await el.dispatchEvent('dragover', { dataTransfer: dt });
  await el.dispatchEvent('drop', { dataTransfer: dt });
}

test('drag-and-drop works on a tool page', async ({ page }) => {
  await page.goto('/tools/merge-pdf');
  await dropFile(page, 'dropzone', 'a.pdf', await samplePdfBuffer(2), 'application/pdf');
  await expect(page.getByTestId('run-button')).toBeEnabled();
  expect(await runAndDownload(page)).toBe('merged.pdf');
});

test('hero quick-start hands a dropped file to a tool', async ({ page }) => {
  await page.goto('/');
  await dropFile(page, 'hero-dropzone', 'report.pdf', await samplePdfBuffer(3), 'application/pdf');
  await page.getByRole('button', { name: 'Merge PDF' }).click();
  await page.waitForURL('**/tools/merge-pdf');
  await expect(page.getByTestId('run-button')).toBeEnabled(); // file auto-loaded via handoff
  expect(await runAndDownload(page)).toBe('merged.pdf');
});

test('home page lists tools', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('PDF tools');
  // Scope to the bento grid (the hero now has its own shortcut chips too).
  const grid = page.locator('#tools-section');
  await expect(grid.getByRole('link', { name: 'Merge PDF' })).toBeVisible();
  await expect(grid.getByRole('link', { name: 'PDF → Text' })).toBeVisible();
});

test('PWA manifest registers an offline service worker', async ({ page, context, request }) => {
  await page.goto('/');
  await expect(page.locator('link[rel="manifest"]')).toHaveAttribute('href', '/manifest.webmanifest');

  const manifestResponse = await request.get('/manifest.webmanifest');
  expect(manifestResponse.ok()).toBe(true);
  const manifest = await manifestResponse.json();
  expect(manifest).toMatchObject({ name: 'Unfleece', start_url: '/', display: 'standalone' });

  const hasActiveWorker = await page.evaluate(async () => {
    if (!('serviceWorker' in navigator)) return false;
    const registration = await navigator.serviceWorker.ready;
    return Boolean(registration.active);
  });
  expect(hasActiveWorker).toBe(true);

  await page.reload();
  await expect
    .poll(() => page.evaluate(() => Boolean(navigator.serviceWorker.controller)), { timeout: 15000 })
    .toBe(true);

  await context.setOffline(true);
  try {
    await page.goto('/uk/tools/merge-pdf');
    await expect(page.getByRole('heading', { level: 1 })).toContainText("Об'єднати PDF");
  } finally {
    await context.setOffline(false);
  }
});

test('localized Ukrainian tool page renders and still runs the tool', async ({ page }) => {
  await page.goto('/uk/tools/merge-pdf');
  await expect(page.locator('html')).toHaveAttribute('lang', 'uk');
  await expect(page.getByRole('heading', { level: 1 })).toContainText("Об'єднати PDF");
  await expect(page.locator('link[rel="alternate"][hreflang="it"]')).toHaveAttribute('href', /\/it\/tools\/merge-pdf$/);
  await expect(page.getByRole('link', { name: 'UK', exact: true })).toHaveAttribute('aria-current', 'page');

  await setFiles(page, [
    pdfFile('a.pdf', await samplePdfBuffer(1)),
    pdfFile('b.pdf', await samplePdfBuffer(1)),
  ]);
  expect(await runAndDownload(page)).toBe('merged.pdf');
});

test('localized Italian tool page and sitemap expose hreflang alternates', async ({ page, request }) => {
  await page.goto('/it/tools/compress-image');
  await expect(page.locator('html')).toHaveAttribute('lang', 'it');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('Comprimi immagine');
  await expect(page.locator('link[rel="alternate"][hreflang="uk"]')).toHaveAttribute('href', /\/uk\/tools\/compress-image$/);
  await expect(page.getByRole('link', { name: 'IT', exact: true })).toHaveAttribute('aria-current', 'page');

  const response = await request.get('/sitemap.xml');
  expect(response.ok()).toBe(true);
  const xml = await response.text();
  expect(xml).toContain('https://unfleece.com/uk/tools/merge-pdf');
  expect(xml).toContain('https://unfleece.com/it/tools/compress-image');
  expect(xml).toContain('hreflang="x-default"');
});

test('crop editor renders the page and applies', async ({ page }) => {
  await page.goto('/tools/crop-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  await page.locator('.pdf-stage-canvas').first().waitFor({ timeout: 20000 });
  expect(await runAndDownload(page)).toContain('cropped');
});

test('sign editor: create in modal, place on multiple pages, zoom', async ({ page }) => {
  await page.goto('/tools/sign-pdf');
  // the signature manager exists before any document is loaded
  await expect(page.getByTestId('create-signature')).toBeVisible();

  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(3))]);
  await page.locator('.pdf-stage-canvas').first().waitFor({ timeout: 20000 });

  // create a signature in the modal — two strokes (full drawing must be captured)
  await page.getByTestId('create-signature').click();
  const box = await page.getByTestId('sign-canvas').boundingBox();
  if (!box) throw new Error('no sign canvas');
  await page.mouse.move(box.x + 20, box.y + box.height * 0.6);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.4, box.y + 10);
  await page.mouse.up();
  await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.8);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.85, box.y + box.height * 0.2);
  await page.mouse.up();
  await page.getByTestId('sig-use').click();

  // modal closes; the finished signature is placed on the current page
  await expect(page.getByTestId('signature-modal')).toHaveCount(0);
  await expect(page.getByTestId('placement')).toHaveCount(1);
  await expect(page.getByTestId('saved-signatures')).toBeVisible(); // saved to the library by default

  // navigate to page 3 and place the saved signature there with a tap
  await page.getByRole('button', { name: 'Next page' }).click();
  await page.getByRole('button', { name: 'Next page' }).click();
  await page.getByTestId('sig-thumb').first().click();
  await expect(page.getByTestId('placement')).toHaveCount(1); // only this page's stamp renders
  await expect(page.getByText('2 placed')).toBeVisible();

  // zoom in grows the rendered page; reset restores
  const w1 = (await page.locator('.pdf-stage-canvas').boundingBox())!.width;
  await page.getByTestId('zoom-in').click();
  await expect
    .poll(async () => (await page.locator('.pdf-stage-canvas').boundingBox())!.width, { timeout: 10000 })
    .toBeGreaterThan(w1 + 10);
  await page.getByTestId('zoom-reset').click();

  // apply both placements in one pass
  expect(await runAndDownload(page)).toContain('signed');
});

test('tapping the page places the signature at that spot', async ({ page }) => {
  await page.goto('/tools/sign-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1))]);
  await page.locator('.pdf-stage-canvas').first().waitFor({ timeout: 20000 });
  // no signature yet → tapping the page opens the create modal
  await page.getByTestId('place-catcher').click({ position: { x: 150, y: 200 } });
  await expect(page.getByTestId('signature-modal')).toBeVisible();
  await page.getByTestId('sig-tab-type').click();
  await page.getByTestId('sig-type-input').fill('Tap Tester');
  await page.getByTestId('sig-use').click();
  await expect(page.getByTestId('placement')).toHaveCount(1); // auto-placed on create
  // tapping the page again drops another stamp right there
  await page.getByTestId('place-catcher').click({ position: { x: 220, y: 120 } });
  await expect(page.getByTestId('placement')).toHaveCount(2);
  expect(await runAndDownload(page)).toContain('signed');
});

test('signature library persists across reloads', async ({ page }) => {
  await page.goto('/tools/sign-pdf');
  await expect(page.getByTestId('sign-tool')).toHaveAttribute('data-ready', 'true');
  await page.getByTestId('create-signature').click();
  await page.getByTestId('sig-tab-type').click();
  await page.getByTestId('sig-type-input').fill('Persist Me');
  await page.getByTestId('sig-use').click();
  await expect(page.getByTestId('sig-thumb')).toHaveCount(1);
  await page.reload();
  // the saved signature must come back from IndexedDB (WebKit regression guard)
  await expect(page.getByTestId('sig-thumb')).toHaveCount(1, { timeout: 10000 });
});

test('sign editor: upload signature image alternative', async ({ page }) => {
  await page.goto('/tools/sign-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1))]);
  await page.locator('.pdf-stage-canvas').first().waitFor({ timeout: 20000 });
  await page.getByTestId('create-signature').click();
  await page.getByTestId('sig-tab-upload').click();
  await page.getByTestId('signature-file-input').setInputFiles([pngFile('signature.png', PNG_1x1)]);
  await expect(page.getByTestId('placement')).toHaveCount(1);
  await expect(page.getByTestId('run-button')).toBeEnabled();
  expect(await runAndDownload(page)).toContain('signed');
});

test('drag a file by its grip to reorder the merge list', async ({ page }) => {
  await page.goto('/tools/merge-pdf');
  await setFiles(page, [
    pdfFile('a.pdf', await samplePdfBuffer(1)),
    pdfFile('b.pdf', await samplePdfBuffer(1)),
    pdfFile('c.pdf', await samplePdfBuffer(1)),
  ]);
  // drag c.pdf (last) up two slots → becomes first
  const grip = page.getByTestId('file-grip').nth(2);
  await grip.scrollIntoViewIfNeeded(); // raw mouse events don't auto-scroll
  const g = await grip.boundingBox();
  if (!g) throw new Error('no grip');
  const rowH = (await page.locator('.file-row').first().boundingBox())!.height + 8;
  await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2);
  await page.mouse.down();
  await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2 - rowH, { steps: 4 });
  await page.mouse.move(g.x + g.width / 2, g.y + g.height / 2 - rowH * 2, { steps: 4 });
  await page.mouse.up();
  await expect(page.locator('.file-row .f-name').first()).toHaveText('c.pdf');
  await expect(page.locator('.file-row .f-name').nth(1)).toHaveText('a.pdf');
});

test('merge two PDFs (and never uploads)', async ({ page }) => {
  const posts: string[] = [];
  page.on('request', (r) => {
    if (r.method() === 'POST' || r.method() === 'PUT') posts.push(r.url());
  });

  await page.goto('/tools/merge-pdf');
  await setFiles(page, [
    pdfFile('a.pdf', await samplePdfBuffer(2)),
    pdfFile('b.pdf', await samplePdfBuffer(3)),
  ]);
  await page.getByRole('button', { name: 'Move b.pdf up' }).click();
  await expect(page.locator('.file-row .f-name').first()).toHaveText('b.pdf');
  await page.getByRole('button', { name: 'Move b.pdf down' }).click();
  await expect(page.locator('.file-row .f-name').first()).toHaveText('a.pdf');
  expect(await runAndDownload(page)).toBe('merged.pdf');

  // Privacy guarantee: nothing was uploaded.
  expect(posts, `unexpected upload requests: ${posts.join(', ')}`).toHaveLength(0);
});

test('invalid PDFs show a focused friendly error', async ({ page }) => {
  await page.goto('/tools/merge-pdf');
  await setFiles(page, [pdfFile('broken.pdf', Buffer.from('not a pdf'))]);
  await page.getByTestId('run-button').click();

  const error = page.getByTestId('error');
  await expect(error).toBeVisible({ timeout: 30000 });
  await expect(error).toBeFocused();
  await expect(error).toContainText('Invalid PDF');
  await expect(error).toContainText('This does not look like a valid PDF');
  await expect(error).toContainText('Nothing left your device');
  await expect(error).not.toContainText('No PDF header');

  await page.getByRole('button', { name: 'Start over' }).click();
  await expect(page.getByTestId('dropzone')).toBeFocused();
});

test('rotate a PDF', async ({ page }) => {
  await page.goto('/tools/rotate-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  expect(await runAndDownload(page)).toContain('rotated');
});

test('split a PDF into a zip', async ({ page }) => {
  await page.goto('/tools/split-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(4))]);
  expect(await runAndDownload(page)).toMatch(/\.zip$/);
});

test('booklet imposition creates a PDF', async ({ page }) => {
  await page.goto('/tools/booklet-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(5))]);
  await page.getByLabel('Sheet size').selectOption('letter');
  expect(await runAndDownload(page)).toBe('doc-booklet.pdf');
});

test('compare two PDFs and reports visual differences', async ({ page }) => {
  await page.goto('/tools/compare-pdfs');
  await setFiles(page, [
    pdfFile('a.pdf', await samplePdfBuffer(1, 'Original')),
    pdfFile('b.pdf', await samplePdfBuffer(1, 'Changed')),
  ]);
  await page.getByTestId('run-button').click();
  await expect(page.getByTestId('result')).toBeFocused({ timeout: 30000 });
  await expect(page.getByTestId('result-text')).toContainText('PDF visual comparison');
  await expect(page.getByTestId('result-text')).toContainText('visual differences');
});

test('extract pages', async ({ page }) => {
  await page.goto('/tools/extract-pdf-pages');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(5))]);
  await page.getByLabel('Pages to extract').fill('1-2,4');
  expect(await runAndDownload(page)).toContain('extracted');
});

test('page range options validate against the selected PDF', async ({ page }) => {
  await page.goto('/tools/extract-pdf-pages');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  await page.getByLabel('Pages to extract').fill('99');

  await expect(page.getByText('This PDF has 2 pages. Pick from 1-2.')).toBeVisible({ timeout: 30000 });
  await expect(page.getByTestId('run-button')).toBeDisabled();

  await page.getByLabel('Pages to extract').fill('1-2');
  await expect(page.getByTestId('run-button')).toBeEnabled();
});

test('add page numbers', async ({ page }) => {
  await page.goto('/tools/add-page-numbers');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(3))]);
  expect(await runAndDownload(page)).toContain('numbered');
});

test('watermark a PDF', async ({ page }) => {
  await page.goto('/tools/watermark-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  expect(await runAndDownload(page)).toContain('watermarked');
});

test('auto-crop margins creates a PDF', async ({ page }) => {
  await page.goto('/tools/auto-crop-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1, 'Centered content'))]);
  await page.getByLabel('Padding kept (pt)').fill('12');
  expect(await runAndDownload(page)).toBe('doc-autocropped.pdf');
});

test('images to PDF', async ({ page }) => {
  await page.goto('/tools/images-to-pdf');
  await setFiles(page, [pngFile('img.png', PNG_1x1)]);
  expect(await runAndDownload(page)).toBe('images.pdf');
});

test('PDF to JPG (pdf.js render) yields a zip', async ({ page }) => {
  await page.goto('/tools/pdf-to-jpg');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  expect(await runAndDownload(page)).toMatch(/\.zip$/);
});

test('PDF to Text extracts the text', async ({ page }) => {
  await page.goto('/tools/pdf-to-text');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1, 'Hello Unfleece'))]);
  await page.getByTestId('run-button').click();
  await expect(page.getByTestId('result')).toBeFocused({ timeout: 30000 });
  await expect(page.getByTestId('result-text')).toContainText('Hello Unfleece', { timeout: 30000 });
});

test('OCR searchable PDF runs Tesseract and returns a searchable PDF', async ({ page }) => {
  await page.goto('/tools/ocr-searchable-pdf');
  await setFiles(page, [pdfFile('scan.pdf', await samplePdfBuffer(1, 'HELLO OCR'))]);
  await page.getByLabel('OCR render scale').fill('2');
  await page.getByTestId('run-button').click();
  await expect(page.getByTestId('result')).toBeFocused({ timeout: 90000 });
  await expect(page.getByTestId('result-text')).toContainText(/HELLO|OCR/, { timeout: 30000 });
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByTestId('download-button').first().click(),
  ]);
  expect(download.suggestedFilename()).toBe('scan-ocr.pdf');
});

test('PDF/A export creates an archival PDF copy with krilla', async ({ page }) => {
  await page.goto('/tools/export-pdfa');
  await setFiles(page, [pdfFile('archive.pdf', await samplePdfBuffer(1, 'Archive me'))]);
  await page.getByLabel('Render scale').fill('1');
  expect(await runAndDownload(page, 60000)).toBe('archive-pdfa.pdf');
});

test('extract PDF text to Word', async ({ page }) => {
  await page.goto('/tools/extract-pdf-to-word');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1, 'Hello Word'))]);
  expect(await runAndDownload(page)).toBe('doc-extracted.docx');
});

test('extract PDF rows to Excel', async ({ page }) => {
  await page.goto('/tools/extract-pdf-to-excel');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1, 'Name  Amount'))]);
  expect(await runAndDownload(page)).toBe('doc-extracted.xlsx');
});

test('extract PDF text to PowerPoint', async ({ page }) => {
  await page.goto('/tools/extract-pdf-to-powerpoint');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2, 'Slide text'))]);
  expect(await runAndDownload(page)).toBe('doc-extracted.pptx');
});

test('convert an image to WebP', async ({ page }) => {
  await page.goto('/tools/convert-image');
  await setFiles(page, [pngFile('img.png', PNG_1x1)]);
  expect(await runAndDownload(page)).toMatch(/\.webp$/);
});

test('convert an image to AVIF with jSquash', async ({ page }) => {
  await page.goto('/tools/convert-image');
  await setFiles(page, [pngFile('img.png', PNG_1x1)]);
  await page.getByLabel('Convert to').selectOption('image/avif');
  expect(await runAndDownload(page)).toBe('img.avif');
});

test('convert a TIFF image through magick-wasm', async ({ page }) => {
  await page.goto('/tools/convert-image');
  await setFiles(page, [tiffFile('scan.tif', tiff1x1Buffer())]);
  await page.getByLabel('Convert to').selectOption('image/png');
  expect(await runAndDownload(page)).toBe('scan.png');
});

test('HTML Markdown to PDF creates a PDF', async ({ page }) => {
  await page.goto('/tools/html-markdown-to-pdf');
  await setFiles(page, [textFile('notes.md', '# Private notes\n\n- merge locally\n- no upload', 'text/markdown')]);
  expect(await runAndDownload(page)).toBe('notes.pdf');
});

test('Bates numbering stamps a PDF', async ({ page }) => {
  await page.goto('/tools/bates-numbering');
  await setFiles(page, [pdfFile('case.pdf', await samplePdfBuffer(2))]);
  await page.getByLabel('Prefix').fill('CASE-');
  expect(await runAndDownload(page)).toBe('case-bates.pdf');
});

test('compress an image to WebP', async ({ page }) => {
  await page.goto('/tools/compress-image');
  await setFiles(page, [pngFile('img.png', PNG_1x1)]);
  expect(await runAndDownload(page)).toBe('img-compressed.webp');
});

test('compress an image to JPEG XL with jSquash', async ({ page }) => {
  await page.goto('/tools/compress-image');
  await setFiles(page, [pngFile('img.png', PNG_1x1)]);
  await page.getByLabel('Output format').selectOption('image/jxl');
  expect(await runAndDownload(page)).toBe('img-compressed.jxl');
});

test('optimize a PDF (Rust→WASM engine)', async ({ page }) => {
  await page.goto('/tools/optimize-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(3))]);
  expect(await runAndDownload(page)).toContain('optimized');
});

test('compress a PDF with Ghostscript WASM', async ({ page }) => {
  await page.goto('/tools/compress-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(2))]);
  expect(await runAndDownload(page, 60000)).toContain('compressed');
});

test('remove metadata', async ({ page }) => {
  await page.goto('/tools/remove-pdf-metadata');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1))]);
  expect(await runAndDownload(page)).toContain('clean');
});

test('protect PDF adds password encryption', async ({ page }) => {
  await page.goto('/tools/protect-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1))]);
  await page.getByLabel('Open password').fill('secret');
  expect(await runAndDownload(page)).toBe('doc-protected.pdf');
});

test('unlock PDF saves an unencrypted copy', async ({ page }) => {
  await page.goto('/tools/unlock-pdf');
  await setFiles(page, [pdfFile('locked.pdf', await protectedPdfBuffer('secret'))]);
  await page.getByLabel('PDF password').fill('secret');
  expect(await runAndDownload(page)).toBe('locked-unlocked.pdf');
});

test('sanitize PDF removes risky extras', async ({ page }) => {
  await page.goto('/tools/sanitize-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1))]);
  await expect(page.getByLabel('Remove annotations')).toBeChecked();
  expect(await runAndDownload(page)).toBe('doc-sanitized.pdf');
});

test('redact PDF burns selected areas into a rebuilt PDF', async ({ page }) => {
  await page.goto('/tools/redact-pdf');
  await setFiles(page, [pdfFile('doc.pdf', await samplePdfBuffer(1, 'Secret text'))]);
  const overlay = page.getByTestId('redact-overlay');
  await expect(overlay).toBeVisible();
  const box = await overlay.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.move(box!.x + 45, box!.y + 45);
  await page.mouse.down();
  await page.mouse.move(box!.x + 190, box!.y + 95);
  await page.mouse.up();
  await expect(page.getByTestId('redaction-count')).toContainText('1');
  expect(await runAndDownload(page)).toBe('doc-redacted.pdf');
});
