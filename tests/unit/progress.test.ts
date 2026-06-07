import { describe, expect, it, vi } from 'vitest';
import { reportProgress, throwIfAborted } from '../../src/lib/progress';
import { runTool } from '../../src/lib/run';
import { TOOLS_BY_ID } from '../../src/lib/registry';

describe('progress helpers', () => {
  it('reports progress to the active sink', () => {
    const onProgress = vi.fn();
    reportProgress(
      { onProgress },
      { phase: 'rendering', label: 'Rendering page 1 of 2…', current: 1, total: 2 },
    );

    expect(onProgress).toHaveBeenCalledWith({
      phase: 'rendering',
      label: 'Rendering page 1 of 2…',
      current: 1,
      total: 2,
    });
  });

  it('throws AbortError for aborted work', () => {
    const controller = new AbortController();
    controller.abort();

    try {
      throwIfAborted(controller.signal);
    } catch (error) {
      expect(error).toBeInstanceOf(DOMException);
      expect((error as DOMException).name).toBe('AbortError');
    }
  });

  it('stops runTool before spawning work when already aborted', async () => {
    const controller = new AbortController();
    controller.abort();
    const file = new File(['%PDF-1.7'], 'doc.pdf', { type: 'application/pdf' });

    await expect(runTool(TOOLS_BY_ID['pdf-to-text'], [file], {}, { signal: controller.signal })).rejects.toMatchObject({
      name: 'AbortError',
    });
  });
});
