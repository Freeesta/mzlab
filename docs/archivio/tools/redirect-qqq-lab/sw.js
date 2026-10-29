// "Switch" service worker for the old address (/QqQ-lab/): apps installed with the old service worker pass to the new site.
// It must NOT delete the caches: the old and the new site have the same origin, the caches of the new one are there too.
self.addEventListener("install", () => { self.skipWaiting(); });
self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    await self.registration.unregister();
    for (const c of clients) {
      try { c.navigate(c.url.replace(/\/QqQ-lab(?=\/|$)/i, "/mzlab")); } catch (e) { /* closed */ }
    }
  })());
});
