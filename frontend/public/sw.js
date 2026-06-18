// Service-worker kill switch.
//
// Older OxyDraw builds vendored the upstream Excalidraw app, which registered a Workbox PWA
// service worker at this path that aggressively cached the app shell + assets. The current
// SPA ships no service worker, so a returning browser would otherwise keep serving the stale
// cached bundle from that old worker. The old worker polls `/sw.js` for updates and finds
// this file: it self-unregisters, clears all caches, and reloads open tabs onto the fresh
// (network) build. Once everyone has loaded it at least once, this file is inert.

self.addEventListener("install", () => self.skipWaiting());

self.addEventListener("activate", (event) => {
  event.waitUntil(
    (async () => {
      const keys = await caches.keys();
      await Promise.all(keys.map((key) => caches.delete(key)));
      await self.registration.unregister();
      const clients = await self.clients.matchAll({ type: "window" });
      for (const client of clients) {
        client.navigate(client.url);
      }
    })(),
  );
});
