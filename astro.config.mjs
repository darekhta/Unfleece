// @ts-check
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

// Unfleece is a fully static, client-side app — no SSR adapter.
// Styling is a hand-authored design system (src/styles), not Tailwind.
export default defineConfig({
  site: 'https://unfleece.com',
  output: 'static',
  integrations: [svelte()],
  vite: {
    worker: { format: 'es' },
    build: { target: 'es2022' },
  },
});
