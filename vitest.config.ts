import { defineConfig } from 'vitest/config';

// Unit tests cover the PURE tool logic (Uint8Array in -> Uint8Array out),
// which runs in Node without a browser. Browser-only paths (pdf.js render,
// Canvas) are covered by Playwright E2E instead.
export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/unit/**/*.test.ts'],
    testTimeout: 30000,
  },
  resolve: {
    alias: {
      '@lib': new URL('./src/lib', import.meta.url).pathname,
      '@components': new URL('./src/components', import.meta.url).pathname,
    },
  },
});
