// @ts-nocheck
/// <reference lib="webworker" />
/* Unfleece offline shell. Handles same-origin GETs only; user files are never requested. */
const CACHE_NAME = 'unfleece-static-v12';
const PRECACHE = [
  '/',
  '/about',
  '/sitemap.xml',
  '/manifest.webmanifest',
  '/logo.png',
  '/og-logo.png',
  '/favicon.svg',
  '/robots.txt',
];

async function sitemapPaths() {
  try {
    const response = await fetch('/sitemap.xml', { cache: 'no-store' });
    if (!response.ok) return [];
    const xml = await response.text();
    return [...xml.matchAll(/<loc>https:\/\/unfleece\.com([^<]+)<\/loc>/g)].map((match) => match[1]);
  } catch (error) {
    return [];
  }
}

async function warmCache() {
  const cache = await caches.open(CACHE_NAME);
  const pages = await sitemapPaths();
  const paths = [...new Set([...PRECACHE, ...pages])];
  await Promise.allSettled(paths.map((path) => cache.add(path)));
}

self.addEventListener('install', (event) => {
  event.waitUntil(warmCache().then(() => self.skipWaiting()));
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((names) => Promise.all(names.filter((name) => name !== CACHE_NAME).map((name) => caches.delete(name))))
      .then(() => self.clients.claim()),
  );
});

async function networkThenCache(request) {
  const cache = await caches.open(CACHE_NAME);
  try {
    const response = await fetch(request);
    if (response.ok) await cache.put(request, response.clone());
    return response;
  } catch (error) {
    return (await cache.match(request)) || (await cache.match('/')) || new Response('Offline', {
      status: 503,
      headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
  }
}

async function cacheThenNetwork(request) {
  const cached = await caches.match(request);
  if (cached) return cached;
  const response = await fetch(request);
  if (response.ok) {
    const cache = await caches.open(CACHE_NAME);
    await cache.put(request, response.clone());
  }
  return response;
}

self.addEventListener('fetch', (event) => {
  const { request } = event;
  if (request.method !== 'GET') return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;

  if (request.mode === 'navigate') {
    event.respondWith(networkThenCache(request));
    return;
  }

  event.respondWith(cacheThenNetwork(request));
});
