// Generates cross-engine test fixtures for the Rust core while pdf-lib is
// still available. Outputs are committed under core/tests/fixtures/ so cargo
// tests can verify compatibility with pdf-lib-produced documents.
import { PDFDocument, StandardFonts } from '@cantoo/pdf-lib';
import { writeFileSync, mkdirSync } from 'node:fs';

mkdirSync('core/tests/fixtures', { recursive: true });
const save = (name, bytes) => { writeFileSync(`core/tests/fixtures/${name}`, bytes); console.log(name, bytes.length, 'bytes'); };

async function basePdf(label) {
  const doc = await PDFDocument.create();
  const font = await doc.embedFont(StandardFonts.Helvetica);
  for (let i = 0; i < 3; i++) {
    const p = doc.addPage([400, 500]);
    p.drawText(`${label} page ${i + 1}`, { x: 40, y: 450, size: 18, font });
  }
  return doc;
}

// 1. Encrypted: user password only
{
  const doc = await basePdf('user-pw');
  doc.encrypt({ userPassword: 'secret123', ownerPassword: 'secret123', permissions: { printing: 'highResolution', copying: true, modifying: true, annotating: true, fillingForms: true, contentAccessibility: true, documentAssembly: true } });
  save('encrypted_user.pdf', await doc.save({ rewrite: true }));
}
// 2. Encrypted: distinct user/owner, restricted permissions
{
  const doc = await basePdf('user-owner');
  doc.encrypt({ userPassword: 'user1', ownerPassword: 'owner1', permissions: { printing: false, copying: false, modifying: false, annotating: false, fillingForms: false, contentAccessibility: false, documentAssembly: false } });
  save('encrypted_user_owner.pdf', await doc.save({ rewrite: true }));
}
// 3. AcroForm with every field type
{
  const doc = await PDFDocument.create();
  const page = doc.addPage([500, 700]);
  const form = doc.getForm();
  const text = form.createTextField('name');
  text.setText('Alice');
  text.addToPage(page, { x: 50, y: 600, width: 200, height: 24 });
  const empty = form.createTextField('email');
  empty.addToPage(page, { x: 50, y: 560, width: 200, height: 24 });
  const multi = form.createTextField('notes');
  multi.enableMultiline();
  multi.setText('line one\nline two');
  multi.addToPage(page, { x: 50, y: 480, width: 220, height: 60 });
  const check = form.createCheckBox('subscribe');
  check.check();
  check.addToPage(page, { x: 50, y: 430, width: 18, height: 18 });
  const check2 = form.createCheckBox('terms');
  check2.addToPage(page, { x: 90, y: 430, width: 18, height: 18 });
  const radio = form.createRadioGroup('color');
  radio.addOptionToPage('red', page, { x: 50, y: 380, width: 18, height: 18 });
  radio.addOptionToPage('blue', page, { x: 90, y: 380, width: 18, height: 18 });
  radio.select('red');
  const drop = form.createDropdown('country');
  drop.setOptions(['Ukraine', 'Italy', 'Other']);
  drop.select('Ukraine');
  drop.addToPage(page, { x: 50, y: 330, width: 160, height: 24 });
  save('acroform.pdf', await doc.save());
}
// 4. The same form FILLED + appearances updated by pdf-lib (appearance parity reference)
{
  const doc = await PDFDocument.create();
  const page = doc.addPage([500, 700]);
  const form = doc.getForm();
  const text = form.createTextField('name');
  text.addToPage(page, { x: 50, y: 600, width: 200, height: 24 });
  text.setText('Filled by pdf-lib');
  form.updateFieldAppearances();
  save('acroform_filled_pdflib.pdf', await doc.save());
}
// 5. pdf-lib flattened form (flatten parity reference)
{
  const doc = await PDFDocument.create();
  const page = doc.addPage([500, 700]);
  const form = doc.getForm();
  const text = form.createTextField('name');
  text.addToPage(page, { x: 50, y: 600, width: 200, height: 24 });
  text.setText('Flattened by pdf-lib');
  form.flatten();
  save('acroform_flattened_pdflib.pdf', await doc.save());
}
console.log('done');
