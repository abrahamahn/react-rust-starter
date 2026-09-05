/* Static assets only. No authentication, API data, note drafts or mutations. */
const PREFIX = 'rrs-static-';
const CACHE = `${PREFIX}v1`;
const STATIC = ['/offline.html', '/offline.css', '/manifest.webmanifest', '/icon.svg', '/icon-192.png', '/icon-512.png'];
self.addEventListener('install', event => {
  event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(STATIC)));
});
self.addEventListener('activate', event => {
  event.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(key => key.startsWith(PREFIX) && key !== CACHE).map(key => caches.delete(key)))).then(() => self.clients.claim()));
});
self.addEventListener('message', event => {
  if (event.data?.type === 'SKIP_WAITING') void self.skipWaiting();
});
self.addEventListener('fetch', event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== 'GET' || url.origin !== self.location.origin || url.pathname.startsWith('/api') || url.pathname.startsWith('/health')) return;
  if (request.mode === 'navigate') {
    event.respondWith(fetch(request).catch(() => caches.match('/offline.html')));
    return;
  }
  const asset = /^\/assets\/[^/]+\.(js|css|woff2|png|svg)$/.test(url.pathname);
  if (!STATIC.includes(url.pathname) && !asset) return;
  event.respondWith(caches.open(CACHE).then(async cache => {
    const cached = await cache.match(request);
    if (cached) return cached;
    const response = await fetch(request);
    if (response.ok && response.type === 'basic') {
      await cache.put(request, response.clone());
      const keys = await cache.keys();
      const dynamic = keys.filter(key => !STATIC.includes(new URL(key.url).pathname));
      for (const key of dynamic.slice(0, Math.max(0, dynamic.length - 64))) await cache.delete(key);
    }
    return response;
  }));
});
