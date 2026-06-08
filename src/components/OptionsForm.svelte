<script lang="ts">
  import Icon from './Icon.svelte';
  import type { OptionField } from '../lib/registry.js';
  import { parsePageRanges } from '../lib/util/ranges.js';

  let {
    fields,
    values = $bindable({}),
    valid = $bindable(true),
    pageCount = null,
  }: { fields: OptionField[]; values?: Record<string, unknown>; valid?: boolean; pageCount?: number | null } = $props();

  function stringValue(name: string): string {
    return String(values[name] ?? '').trim();
  }

  function isVisible(f: OptionField): boolean {
    return !f.showWhen || values[f.showWhen.name] === f.showWhen.value;
  }

  function rangeError(raw: string, emptyMessage: string): string {
    if (!raw) return emptyMessage;
    if (!pageCount) return '';
    try {
      parsePageRanges(raw, pageCount);
      return '';
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      return message.includes('out of range')
        ? `This PDF has ${pageCount} pages. Pick from 1-${pageCount}.`
        : 'Enter pages like 1-3, 5.';
    }
  }

  function reorderError(raw: string): string {
    if (!raw) return 'Enter every page number once, like 3,1,2.';
    if (!pageCount) return '';
    const order = raw.split(',').map((s) => Number(s.trim()));
    if (order.some((n) => !Number.isInteger(n))) return 'Enter comma-separated page numbers.';
    if (order.length !== pageCount) return `Enter each page number once (1-${pageCount}).`;
    const seen = new Set(order);
    if (seen.size !== order.length) return 'Remove duplicate page numbers.';
    for (let i = 1; i <= pageCount; i++) {
      if (!seen.has(i)) return `Include every page from 1-${pageCount}.`;
    }
    return '';
  }

  function validateField(f: OptionField): string {
    const raw = stringValue(f.name);
    if (f.type === 'pages') {
      const blankMeansAll = f.label.toLowerCase().includes('blank = all');
      return blankMeansAll && !raw ? '' : rangeError(raw, 'Enter pages like 1-3, 5.');
    }
    if (f.name === 'ranges' && values.mode === 'ranges') {
      if (!raw) return 'Enter at least one range, like 1-3, 4-6.';
      if (!pageCount) return '';
      const tokens = raw.split(',').map((s) => s.trim()).filter(Boolean);
      if (tokens.length === 0) return 'Enter at least one range, like 1-3, 4-6.';
      for (const token of tokens) {
        const error = rangeError(token, 'Enter at least one range, like 1-3, 4-6.');
        if (error) return error;
      }
      return '';
    }
    if (f.name === 'order') return reorderError(raw);
    return '';
  }

  const errors = $derived.by(() => Object.fromEntries(
    fields
      .filter(isVisible)
      .map((field) => [field.name, validateField(field)] as const)
      .filter(([, error]) => error),
  ));

  const visibleFields = $derived(fields.filter(isVisible));

  $effect(() => {
    valid = Object.keys(errors).length === 0;
  });
</script>

{#if visibleFields.length > 0}
  <form class="options-panel" aria-label="Tool options" onsubmit={(e) => e.preventDefault()}>
    <h3>Options</h3>
    <div class="options-grid">
      {#each visibleFields as f (f.name)}
        {#if f.type === 'checkbox'}
          <div class="span-2">
            <label class="checkbox-row">
              <input type="checkbox" bind:checked={values[f.name]} />
              <span class="checkbox-box"><Icon name="check" /></span>
              <span>{f.label}</span>
            </label>
          </div>
        {:else}
          {@const error = errors[f.name]}
          {@const errorId = `opt-${f.name}-error`}
          <div class={f.type === 'text' || f.type === 'pages' ? 'span-2' : ''}>
            <label class="label" for={`opt-${f.name}`}>
              {f.label}{#if f.type === 'text' && f.required}<span class="req" aria-hidden="true"> *</span>{/if}
            </label>

            {#if f.type === 'select'}
              <select id={`opt-${f.name}`} class="field {error ? 'invalid' : ''}" bind:value={values[f.name]} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? errorId : undefined}>
                {#each f.options as opt (opt.value)}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            {:else if f.type === 'number'}
              <input id={`opt-${f.name}`} class="field {error ? 'invalid' : ''}" type="number" min={f.min} max={f.max} step={f.step ?? 1} bind:value={values[f.name]} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? errorId : undefined} />
            {:else}
              <input id={`opt-${f.name}`} class="field {error ? 'invalid' : ''}" type="text" placeholder={'placeholder' in f ? f.placeholder : ''} bind:value={values[f.name]} aria-invalid={error ? 'true' : undefined} aria-describedby={error ? errorId : undefined} />
            {/if}

            {#if error}
              <div class="field-error" id={errorId} role="status"><Icon name="alert" /> {error}</div>
            {/if}

            {#if 'help' in f && f.help}
              <div class="help">{f.help}</div>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  </form>
{/if}
