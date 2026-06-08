export type RunPhase = 'loading' | 'rendering' | 'extracting' | 'compressing' | 'zipping' | 'optimizing' | 'working' | 'saving';

export interface RunProgress {
  phase: RunPhase;
  label: string;
  current?: number;
  total?: number;
}

export interface RunOptions {
  signal?: AbortSignal;
  onProgress?: (progress: RunProgress) => void;
}

export type ProgressCallback = (progress: RunProgress) => void;

export function notifyProgress(callback: ProgressCallback | undefined, progress: RunProgress): void {
  callback?.(progress);
}

export function abortError(): DOMException {
  return new DOMException('Operation cancelled', 'AbortError');
}

export function throwIfAborted(signal?: AbortSignal): void {
  if (!signal?.aborted) return;
  if (signal.reason instanceof Error) throw signal.reason;
  throw abortError();
}

export function reportProgress(options: RunOptions | undefined, progress: RunProgress): void {
  throwIfAborted(options?.signal);
  options?.onProgress?.(progress);
}
