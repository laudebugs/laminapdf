/// <reference lib="webworker" />
// The reader's service worker (ADR 0011 §1 and §4): after one visit the reader works with no
// network. At install it caches the app shell and the WebAssembly the reader loads; it serves them
// from that cache until the user agrees to an update, so a deploy never swaps the reader under an
// open document. It answers only same-origin GET requests: a document linked with `#url=` lives on
// another site and is never seen, let alone cached, here. Cached responses keep the headers they
// arrived with (the Cache API stores them), so the shell served offline carries the same CSP,
// frame-ancestors and other ADR 0011 headers as the one served online.
//
// The build (plugins/pwa.ts) compiles this file and puts the precache list and the version, a hash
// of every precached file, in front of it, so a deploy that changes any file installs a new worker.

declare const self: ServiceWorkerGlobalScope;
/** The paths installed with this version: `/` (the shell) and every hashed asset the reader loads. */
declare const __LAMINA_PRECACHE__: readonly string[];
/** A hash of the precached files' contents. */
declare const __LAMINA_VERSION__: string;

/** This version's shell and assets. */
const SHELL = `lamina-shell-${__LAMINA_VERSION__}`;
/** Same-origin assets left out of the precache (the hayro backend), cached on first use. */
const LAZY_ASSETS = `lamina-lazy-${__LAMINA_VERSION__}`;

self.addEventListener("install", (event) => {
  // `reload`: the files come from the server, never from an HTTP cache that might hold older ones.
  event.waitUntil(
    caches
      .open(SHELL)
      .then((cache) => cache.addAll(__LAMINA_PRECACHE__.map((path) => new Request(path, { cache: "reload" })))),
  );
  // No skipWaiting here: a new version waits until the user chooses Reload (R15).
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      for (const name of await caches.keys()) {
        if (name.startsWith("lamina-") && name !== SHELL && name !== LAZY_ASSETS) await caches.delete(name);
      }
      // The first install controls the page that installed it, so it works offline at once.
      await self.clients.claim();
    })(),
  );
});

self.addEventListener("message", (event) => {
  // The page sends this only when the user chose Reload.
  if (event.data === "skip-waiting") void self.skipWaiting();
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  const url = new URL(request.url);
  // Another site's request (a linked document, a link the user opens) goes to the network untouched.
  if (request.method !== "GET" || url.origin !== self.location.origin) return;
  event.respondWith(request.mode === "navigate" ? shell(request) : asset(request, url));
});

/** Every page of the reader is the one shell, with the headers it was cached with. */
async function shell(request: Request): Promise<Response> {
  return (await caches.match("/", { cacheName: SHELL })) ?? fetch(request);
}

/** A cached asset, else the network; a hashed asset fetched now is kept for next time. */
async function asset(request: Request, url: URL): Promise<Response> {
  const cached =
    (await caches.match(request, { cacheName: SHELL })) ?? (await caches.match(request, { cacheName: LAZY_ASSETS }));
  if (cached !== undefined) return cached;
  const response = await fetch(request);
  // Hashed names never change, so a copy can't go stale.
  if (response.ok && url.pathname.startsWith("/assets/")) {
    const copy = response.clone();
    void caches.open(LAZY_ASSETS).then((cache) => cache.put(request, copy));
  }
  return response;
}
