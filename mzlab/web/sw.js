// @ts-strict
// Service worker of the browser version (added to the site root by tools/build_site.py; the local program does not use it).
// App files are cache-first: the cache name carries the hash of the whole site, so a file in it is the right version by construction and
// needs no revalidation. A miss is fetched, checked against the SHA-256 of static/sw-manifest.json and stored; the cache is also filled in the
// background from that manifest. Navigations are network-first (short timeout), so a new deploy is seen at the next page load.
// The big, rarely changing files (Ketcher; the Python of mzFinder) live in their own cache. Offline, everything comes from the caches.
try { importScripts("static/nomi.js"); } catch (_) { /* local testing */ }
const APPV = "__APP__";
const APP = (typeof NOMI !== "undefined" ? NOMI.CACHE_APP : "mzlab-app-") + APPV, BIG = (typeof NOMI !== "undefined" ? NOMI.CACHE_BIG : "mzlab-big-") + "__BIG__";
/** @type {ServiceWorkerGlobalScope} */
const sw = /** @type {any} */ (self);
const BASE = new URL("./", location.href);                       // the site root: this file sits there
const NC = { cache: /** @type {RequestCache} */ ("no-cache") };  // revalidate: GitHub Pages sends max-age=600
const NAV_MS = 3000, FILL_AFTER_MS = 2500, FILL_PARALLEL = 6;
const MARK = new URL(".fill-done", BASE).href;                   // present in the app cache once the background fill finished
const NEVER = new Set(["static/manifest.json", "static/build.json", "static/sw-manifest.json"]);   // answer for the build itself: always from the network
const T0 = Date.now();
let stale = false;     // the site on the network is a newer version than this worker: the cache is bypassed until the new worker takes over
/** @type {Promise<void> | null} */
let fill = null;
// Cross-origin isolation (SharedArrayBuffer) without server headers: every same-origin response gets COOP/COEP/CORP, also from the cache.
// Everything of the site is same-origin; cross-origin fetches of the page are CORS requests, which require-corp does not block.
/** @param {Response} res */
const isolata = res => {
  if (res.status === 0 || res.type === "opaqueredirect") return res;
  const h = new Headers(res.headers);
  h.set("Cross-Origin-Embedder-Policy", "require-corp"); h.set("Cross-Origin-Opener-Policy", "same-origin"); h.set("Cross-Origin-Resource-Policy", "same-origin");
  return new Response(res.body, { status: res.status, statusText: res.statusText, headers: h });
};
/** SHA-256 (hex) of the body of a response, which stays readable. */
const shaDe = async (/** @type {Response} */ r) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", await r.clone().arrayBuffer()))].map(b => b.toString(16).padStart(2, "0")).join("");
/** Path of a URL under the site root as the manifest writes it; the folders answer with their index.html. */
const rel = (/** @type {URL} */ u) => {
  let p = u.pathname.slice(BASE.pathname.length);
  try { p = decodeURIComponent(p); } catch (_) { /* keep as is */ }
  return p === "" || p.endsWith("/") ? p + "index.html" : p;
};
/** @type {Promise<Record<string, string> | null> | null} */
let manP = null;
/** @type {Set<string>} */
let nofill = new Set();
/** Path -> SHA-256 of the files of THIS version (null: manifest unreachable, or it describes another version, which sets `stale`). */
const manifestoSw = () => manP || (manP = (async () => {
  try {
    const r = await fetch(new URL("static/sw-manifest.json", BASE).href, NC);
    if (!r.ok) throw new Error("manifest");
    const j = await r.json();
    if (j.app !== APPV) { stale = true; sw.registration.update().catch(() => {}); return null; }
    stale = false;
    /** @type {Record<string, string>} */
    const out = {};
    nofill = new Set(j.nofill || []);
    for (const [k, v] of Object.entries(j.files || {})) out[k] = String(v);
    return out;
  } catch (_) { manP = null; return null; }
})());
/** Expected SHA-256 of the cacheable file `u`, or undefined (not part of this build, never cached, library, vendor, API). */
const expected = async (/** @type {URL} */ u) => {
  const p = rel(u);
  if (NEVER.has(p) || p.startsWith("static/librerie/") || p.startsWith("static/vendor/") || u.pathname.includes("/api/")) return undefined;
  return ((await manifestoSw()) || {})[p];
};
/** Fetches `req`; `ok` is true when the body is the one the manifest promises (HTTP cache first, then a revalidation). */
const traer = async (/** @type {Request} */ req, /** @type {string | undefined} */ want, fresh = false) => {
  if (!want) return { res: await fetch(req, NC), ok: false };
  let res = fresh ? await fetch(req, NC) : await fetch(req);
  if (res.status === 200 && await shaDe(res) === want) return { res, ok: true };
  res = await fetch(req, NC);
  return { res, ok: res.status === 200 && await shaDe(res) === want };
};
/** The app cache, opened once: opening it for each of the ~400 requests of a load is a queue of its own. */
/** @type {Promise<Cache> | null} */
let appCache = null;
const abrirApp = () => appCache || (appCache = caches.open(APP).catch(err => { appCache = null; throw err; }));
/** The copy in `cache`; a URL with a query is looked up ignoring it (the query only busts browser caches), which scans the cache, so only then. */
const buscar = (/** @type {Cache} */ cache, /** @type {Request} */ req) => cache.match(req, new URL(req.url).search ? { ignoreSearch: true } : undefined);
const guardar = async (/** @type {Cache} */ cache, /** @type {Request} */ req, /** @type {Response} */ res) => {
  try { await cache.put(req, res.clone()); } catch (_) { /* storage full: serve anyway */ }
};
/** Fills the app cache with every file of the manifest the page has not loaded yet (a few at a time, not blocking anything). */
const llenar = async () => {
  try {
    const cache = await abrirApp();
    if (await cache.match(MARK)) return;
    const man = await manifestoSw();
    if (!man) return;
    const todo = Object.keys(man).filter(p => !nofill.has(p));
    const have = new Set((await cache.keys()).map(r => r.url));       // one listing: asking the cache once per file is slow
    let failed = 0;
    const worker = async () => {
      for (let p = todo.pop(); p !== undefined; p = todo.pop()) {
        try {
          const req = new Request(new URL(p.split("/").map(encodeURIComponent).join("/"), BASE).href);
          if (stale) { failed++; continue; }
          if (have.has(req.url)) continue;
          const { res, ok } = await traer(req, man[p]);
          if (ok && !stale) {
            await guardar(cache, req, res);
            if (p.endsWith("index.html")) await guardar(cache, new Request(new URL(p.slice(0, -"index.html".length), BASE).href), res);   // the page is also asked for as its folder
          } else failed++;
        } catch (_) { failed++; }
      }
    };
    await Promise.all(Array.from({ length: FILL_PARALLEL }, worker));
    if (!failed && !stale) await cache.put(MARK, new Response("ok"));
  } catch (_) { /* retried when the worker starts again */ }
};
let filled = false;
/** Starts the fill once, FILL_AFTER_MS after the worker started, so the page's own requests go first. */
const arrancar = () => fill || (fill = new Promise(r => setTimeout(r, Math.max(0, FILL_AFTER_MS - (Date.now() - T0)))).then(llenar).then(() => { filled = true; }));
sw.addEventListener("install", () => {
  sw.skipWaiting();
  arrancar();                                                          // not in waitUntil: the first page must not wait for it
});
sw.addEventListener("activate", (/** @type {ExtendableEvent} */ e) => e.waitUntil((async () => {
  const pfx = typeof NOMI !== "undefined" ? NOMI.CACHE_PREFIX : "mzlab-";
  for (const k of await caches.keys()) {
    if (k.startsWith("qqq-") || (k.startsWith(pfx) && k !== APP && k !== BIG)) {
      await caches.delete(k);
    }
  }
  await sw.clients.claim();
})()));
/** An app file: from the cache of this version; a miss goes to the network and is stored once it matches the manifest. */
const app = async (/** @type {Request} */ req) => {
  const cache = await abrirApp();
  if (!stale) { const hit = await buscar(cache, req); if (hit) return isolata(hit); }
  try {
    const { res, ok } = await traer(req, await expected(new URL(req.url)));
    if (ok && !stale) await guardar(cache, req, res);
    return isolata(res);
  } catch (err) {
    const hit = await buscar(cache, req);
    if (hit) return isolata(hit);
    throw err;
  }
};
/** A page load: the network first (a new deploy shows at once), the cached copy when it is slow or offline. */
const navegar = async (/** @type {Request} */ req) => {
  const cache = await abrirApp();
  manP = null;                    // the manifest too is asked again: it tells whether the site changed since this worker started
  const net = (async () => {
    const { res, ok } = await traer(req, await expected(new URL(req.url)), true);
    if (ok && !stale) await guardar(cache, req, res);
    return res;
  })();
  const hit = await buscar(cache, req);
  if (!hit) return isolata(await net);
  net.catch(() => {});
  try {
    const res = await Promise.race([net, new Promise(r => setTimeout(r, NAV_MS, null))]);
    if (res) return isolata(res);
  } catch (_) { /* offline */ }
  return isolata(hit);
};
/** A big file (Ketcher, mzFinder's Python): the cache wins, the network fills it. */
const grande = async (/** @type {Request} */ req) => {
  const cache = await caches.open(BIG);
  const hit = await cache.match(req);
  if (hit) return isolata(hit);
  try {
    const res = await fetch(req);
    if (res.status === 200) await guardar(cache, req, res);
    return isolata(res);
  } catch (err) {
    const old = await cache.match(req, { ignoreSearch: true });
    if (old) return isolata(old);
    throw err;
  }
};
sw.addEventListener("fetch", (/** @type {FetchEvent} */ e) => {
  const req = e.request, url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== location.origin || req.headers.has("range")) return;
  if (req.cache === "only-if-cached" && req.mode !== "same-origin") return;
  if (url.pathname.includes("/static/librerie/")) return;           // the spectral-library packages go to the folder of the user, not to a second copy in the cache
  if (url.pathname.includes("/api/")) return;                       // answers of the engine and files of the user are never cached
  const big = /\/static\/vendor\//.test(url.pathname);
  if (!big && !filled) e.waitUntil(arrancar());                     // keeps the worker alive while it fills; one that was stopped finishes what is missing
  e.respondWith(big ? grande(req) : req.mode === "navigate" ? navegar(req) : app(req));
});
