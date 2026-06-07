import { tick } from 'svelte';

export async function focusAfterUpdate(resolveElement: () => HTMLElement | null | undefined): Promise<void> {
  await tick();
  resolveElement()?.focus({ preventScroll: false });
}

export async function focusSelectorAfterUpdate(selector: string): Promise<void> {
  await focusAfterUpdate(() =>
    typeof document === 'undefined'
      ? null
      : document.querySelector<HTMLElement>(selector),
  );
}

export function focusDropzoneAfterUpdate(): Promise<void> {
  return focusSelectorAfterUpdate('[data-testid="dropzone"]');
}
