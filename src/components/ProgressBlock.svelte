<script lang="ts">
  import type { RunProgress } from '../lib/progress.js';

  let {
    progress = null,
    fallback = 'Working on your file…',
  }: { progress?: RunProgress | null; fallback?: string } = $props();

  const progressPct = $derived(
    progress?.total && progress.current !== undefined
      ? Math.max(0, Math.min(100, (progress.current / progress.total) * 100))
      : null,
  );
  const label = $derived(progress?.label ?? fallback);
</script>

<div class="progress-block" role="status" aria-live="polite" aria-atomic="true">
  <div class="progress-label">
    <span class="spinner" style="color:var(--accent-400)" aria-hidden="true"></span>
    <span>{label}</span>
    {#if progressPct !== null}<span class="progress-pct">{Math.round(progressPct)}%</span>{/if}
  </div>
  <div
    class="progress-track {progressPct === null ? 'indeterminate' : ''}"
    role="progressbar"
    aria-valuemin="0"
    aria-valuemax={progressPct === null ? undefined : 100}
    aria-valuenow={progressPct === null ? undefined : Math.round(progressPct)}
    aria-valuetext={label}
    aria-label={label}
  >
    <span class="progress-fill" style={progressPct === null ? '' : `width:${progressPct}%`}></span>
  </div>
</div>
