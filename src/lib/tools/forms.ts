// AcroForm field operations, executed by the Rust→WASM core (lopdf): list,
// fill (with regenerated appearance streams) and flatten. No pdf-lib.
import { wasmListFormFields, wasmFillForm, wasmFlattenForm, type WasmFormField } from '../wasm/core.js';

export type FormField = WasmFormField;
export type FormValues = Record<string, string | boolean>;

/** List the fillable fields in an AcroForm. */
export async function listFormFields(bytes: Uint8Array): Promise<FormField[]> {
  return wasmListFormFields(bytes);
}

/** Fill AcroForm fields by name. Strings fill text/choice; booleans toggle checkboxes. */
export async function fillForm(bytes: Uint8Array, values: FormValues): Promise<Uint8Array> {
  return wasmFillForm(bytes, values);
}

/** Flatten the form, baking field values into static page content. */
export async function flattenForm(bytes: Uint8Array): Promise<Uint8Array> {
  return wasmFlattenForm(bytes);
}
