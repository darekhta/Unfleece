<script lang="ts">
  import { iconPaths } from '../lib/icons.js';

  let {
    name,
    sw = 1.75,
    size = undefined,
    class: cls = '',
  }: { name: string; sw?: number; size?: number; class?: string } = $props();
</script>

<svg
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={sw}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  class={('icon ' + cls).trim()}
  style={size ? `width:${size}px;height:${size}px` : ''}>{@html iconPaths(name)}</svg>
