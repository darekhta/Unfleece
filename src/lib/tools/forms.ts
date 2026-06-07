import { loadPdf } from '../util/pdf.js';

export interface FormField {
  name: string;
  type: 'text' | 'checkbox' | 'radio' | 'dropdown' | 'optionlist' | 'button' | 'signature' | 'unknown';
  options?: string[];
  value?: string | boolean;
}

/** List the fillable fields in an AcroForm. */
export async function listFormFields(bytes: Uint8Array): Promise<FormField[]> {
  const doc = await loadPdf(bytes);
  const form = doc.getForm();
  return form.getFields().map((f) => {
    const ctor = f.constructor.name;
    const type: FormField['type'] =
      ctor === 'PDFTextField'
        ? 'text'
        : ctor === 'PDFCheckBox'
          ? 'checkbox'
          : ctor === 'PDFRadioGroup'
            ? 'radio'
            : ctor === 'PDFDropdown'
              ? 'dropdown'
              : ctor === 'PDFOptionList'
                ? 'optionlist'
                : ctor === 'PDFButton'
                  ? 'button'
                  : ctor === 'PDFSignature'
                    ? 'signature'
                    : 'unknown';
    const name = f.getName();
    if (type === 'text') {
      return { name, type, value: form.getTextField(name).getText() ?? '' };
    }
    if (type === 'checkbox') {
      return { name, type, value: form.getCheckBox(name).isChecked() };
    }
    if (type === 'dropdown') {
      const dropdown = form.getDropdown(name);
      return { name, type, options: dropdown.getOptions(), value: dropdown.getSelected()[0] ?? '' };
    }
    if (type === 'radio') {
      const radio = form.getRadioGroup(name);
      return { name, type, options: radio.getOptions(), value: radio.getSelected() ?? '' };
    }
    if (type === 'optionlist') {
      const list = form.getOptionList(name);
      return { name, type, options: list.getOptions(), value: list.getSelected()[0] ?? '' };
    }
    return { name, type };
  });
}

export type FormValues = Record<string, string | boolean>;

/** Fill AcroForm fields by name. Strings fill text/dropdown; booleans toggle checkboxes. */
export async function fillForm(bytes: Uint8Array, values: FormValues): Promise<Uint8Array> {
  const doc = await loadPdf(bytes);
  const form = doc.getForm();

  for (const [name, value] of Object.entries(values)) {
    const field = form.getFieldMaybe(name);
    if (!field) continue;
    const ctor = field.constructor.name;
    if (ctor === 'PDFTextField') {
      form.getTextField(name).setText(String(value));
    } else if (ctor === 'PDFCheckBox') {
      const cb = form.getCheckBox(name);
      if (value) cb.check();
      else cb.uncheck();
    } else if (ctor === 'PDFDropdown') {
      form.getDropdown(name).select(String(value));
    } else if (ctor === 'PDFRadioGroup') {
      form.getRadioGroup(name).select(String(value));
    } else if (ctor === 'PDFOptionList') {
      form.getOptionList(name).select(String(value));
    }
  }
  return doc.save();
}

/** Flatten the form, baking field values into static page content. */
export async function flattenForm(bytes: Uint8Array): Promise<Uint8Array> {
  const doc = await loadPdf(bytes);
  doc.getForm().flatten();
  return doc.save();
}
