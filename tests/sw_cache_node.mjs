// Loads mzlab/web/sw.js with a mocked service-worker scope: app files are cache-first by site version (hit, miss -> network -> checked against
// the manifest -> put), navigations are network-first, a worker of an older version answers from its cache only the files the newer one did not change, the background fill, unchanged files carried over from the previous version, old caches go.
import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const src = fs.readFileSync(path.join(root, "mzlab/web/sw.js"), "utf8").replace("__APP__", "abc").replace("__BIG__", "v-1");
const sha = s => crypto.createHash("sha256").update(s).digest("hex");
const O = "https://x.test/";
const FILES = { "index.html": "<html>", "static/app.js": "app()", "static/lazy.js": "lazy()", "static/big.wasm": "wasm", "static/esempi/a.mzML": "mz" };
const manifest = (over = {}) => JSON.stringify({ app: "abc", files: Object.fromEntries(Object.entries(FILES).map(([k, v]) => [k, sha(v)])), nofill: ["static/esempi/a.mzML"], ...over });

/** Runs sw.js; `net` maps a path to its body (undefined: 404, "throw": offline). */
function mondo(net = { ...FILES }, man = manifest()) {
  const listeners = {}, timers = [], stores = new Map(), st = { fetches: [], updates: 0, waits: 0, offline: false, net };
  const key = (r, o) => { const u = new URL(typeof r === "string" ? r : r.url); if (o?.ignoreSearch) u.search = ""; return u.href; };
  const open = async name => {
    if (!stores.has(name)) stores.set(name, new Map());
    const m = stores.get(name);
    return {
      async match(r, o) { for (const [k, v] of m) if (key(k, o) === key(r, o)) return v.clone(); },
      async put(r, res) { m.set(key(r), res.clone()); },
      async keys() { return [...m.keys()].map(url => ({ url })); },
    };
  };
  const sw = { addEventListener: (n, f) => { listeners[n] = f; }, skipWaiting() {}, clients: { claim() {} }, registration: { update: async () => { st.updates++; } } };
  const ctx = {
    self: sw, location: { origin: "https://x.test", href: O + "sw.js" }, URL, Headers, Response, Request, crypto: crypto.webcrypto, importScripts() {},
    setTimeout: (f, ms) => { f.ms = ms; timers.push(f); return timers.length; },
    caches: { open, keys: async () => [...stores.keys()], delete: async n => stores.delete(n) },
    fetch: async (r, init) => {
      const u = new URL(typeof r === "string" ? r : r.url);
      st.fetches.push(u.pathname.slice(1));
      if (st.offline) throw new TypeError("offline");
      if (u.pathname === "/static/sw-manifest.json") return st.hang ? new Promise(() => {}) : new Response(man, { status: 200 });
      const body = st.net[u.pathname.slice(1) || "index.html"];
      return body === undefined ? new Response("", { status: 404 }) : new Response(body, { status: 200 });
    },
  };
  vm.runInNewContext(src, ctx);
  const get = async (p, extra = {}) => {
    let out; const later = [];
    const e = { request: { method: "GET", url: O + p, headers: new Headers(), cache: "default", mode: "same-origin", ...extra }, respondWith: x => { out = x; }, waitUntil(x) { later.push(x); st.waits++; } };
    listeners.fetch(e);
    if (!out) return null;
    const res = await out;
    // the store of a miss runs after the answer (waitUntil): settled here so the next request sees it (the fill waits for a timer the mock never fires)
    await Promise.all(later.map(x => Promise.race([x, new Promise(r => setTimeout(r, 20))])));
    assert.equal(res.headers.get("Cross-Origin-Embedder-Policy"), "require-corp");
    assert.equal(res.headers.get("Cache-Control"), "no-cache", "the browser must ask the worker every time");
    return res;
  };
  const netCount = p => st.fetches.filter(f => f === p).length;
  return { get, st, stores, listeners, timers, netCount, ctx };
}

{ // miss -> network -> checked -> stored; then a hit with no network at all
  const w = mondo();
  assert.equal(await (await w.get("static/app.js")).text(), "app()");
  const n = w.st.fetches.length;
  assert.equal(await (await w.get("static/app.js")).text(), "app()");
  assert.equal(w.st.fetches.length, n, "second request: cache hit, no network " + w.st.fetches);
  assert.equal(await (await w.get("static/app.js?v=3")).text(), "app()", "a query string is the same file of the same version");
  assert.equal(w.st.fetches.length, n);
}
{ // a miss is answered without waiting for its store (a cold load is dozens of misses before the first paint)
  const w = mondo();
  const cache = await w.ctx.caches.open("mzlab-app-abc");
  let release; const held = new Promise(r => { release = r; });
  const put = cache.put; cache.put = async (r, res) => { await held; return put(r, res); };
  w.ctx.caches.open = async () => cache;
  const res = await Promise.race([w.get("static/app.js"), new Promise(r => setTimeout(r, 200, null))]);
  assert.ok(res, "answered while the store is still pending");
  assert.equal(await res.text(), "app()");
  release();
}
{ // cache-first: a cached file is never revalidated
  const w = mondo();
  await w.get("static/app.js"); w.st.net["static/app.js"] = "changed()";
  assert.equal(await (await w.get("static/app.js")).text(), "app()");
  assert.equal(w.netCount("static/app.js"), 1);
}
{ // a body that is not the one of the manifest is served but never stored
  const w = mondo({ ...FILES, "static/app.js": "other()" });
  assert.equal(await (await w.get("static/app.js")).text(), "other()");
  w.st.net["static/app.js"] = "app()";
  await w.get("static/app.js");
  assert.equal(w.netCount("static/app.js"), 3, "first try (HTTP cache) + revalidation, then a new miss");
}
{ // not part of the build: network, no store
  const w = mondo({ ...FILES, "static/extra.js": "x" });
  await w.get("static/extra.js"); await w.get("static/extra.js");
  assert.equal(w.netCount("static/extra.js"), 2);
}
{ // the engine's answers, libraries and non-GET are not touched
  const w = mondo();
  assert.equal(await w.get("api/files"), null);
  assert.equal(await w.get("static/librerie/x.json"), null);
  assert.equal(await w.get("static/app.js", { method: "POST" }), null);
  assert.equal(w.st.fetches.length, 0);
}
{ // navigation: network first, so a deploy shows at once; offline: the cached page
  const w = mondo();
  assert.equal(await (await w.get("", { mode: "navigate" })).text(), "<html>");
  assert.equal(w.netCount(""), 1);
  await w.get("", { mode: "navigate" });
  assert.equal(w.netCount(""), 2, "every navigation asks the network");
  await w.get("static/app.js"); w.st.offline = true;
  assert.equal(await (await w.get("", { mode: "navigate" })).text(), "<html>", "offline: from the cache");
  assert.equal(await (await w.get("static/app.js")).text(), "app()", "offline: app file from the cache");
}
{ // the site on the network is another version: the worker asks for an update; a cached file answers only when its bytes are the newer version's
  const NEW = { ...FILES, "static/lazy.js": "lazy() v2" };
  const w = mondo({ ...NEW }, manifest({ app: "zzz", files: Object.fromEntries(Object.entries(NEW).map(([k, v]) => [k, sha(v)])) }));
  const mine = await w.ctx.caches.open("mzlab-app-abc");
  for (const p of ["static/app.js", "static/lazy.js"]) await mine.put(new Request(O + p), new Response(FILES[p], { status: 200 }));
  await w.get("static/extra.js");
  assert.equal(w.st.updates, 1);
  assert.equal(await (await w.get("static/app.js")).text(), "app()");
  assert.equal(w.netCount("static/app.js"), 0, "unchanged in the newer version: from the cache " + w.st.fetches);
  assert.equal(await (await w.get("static/lazy.js")).text(), "lazy() v2", "changed: the newer bytes, never the cached older ones");
  await w.get("static/lazy.js"); await w.get("static/big.wasm"); await w.get("static/big.wasm");
  assert.equal(w.netCount("static/lazy.js"), 2, "stale: nothing stored");
  assert.equal(w.netCount("static/big.wasm"), 2, "stale: a miss is not stored");
}
{ // a worker of an older version that the browser stopped and started again: its first request asks the manifest before the cache answers
  const NEW = { ...FILES, "static/lazy.js": "lazy() v2" };
  const w = mondo({ ...NEW }, manifest({ app: "zzz", files: Object.fromEntries(Object.entries(NEW).map(([k, v]) => [k, sha(v)])) }));
  await (await w.ctx.caches.open("mzlab-app-abc")).put(new Request(O + "static/lazy.js"), new Response("lazy()", { status: 200 }));
  assert.equal(await (await w.get("static/lazy.js")).text(), "lazy() v2", "never the older bytes to a page of the newer version");
  const n = w.st.waits;
  await w.get("static/app.js");
  assert.equal(w.st.waits, n, "stale: no event kept open (the newer worker activates only when the older one has none)");
}
{ // a hung network (not offline): a restarted worker waits for the manifest at most as long as a page (3 s), then its cache answers
  const w = mondo();
  await (await w.ctx.caches.open("mzlab-app-abc")).put(new Request(O + "static/lazy.js"), new Response("lazy()", { status: 200 }));
  w.st.hang = true;
  const p = w.get("static/lazy.js");
  await new Promise(r => setTimeout(r, 20));
  const t = w.timers.find(f => f.ms === 3000);
  assert.ok(t, "the wait for the manifest has a timeout");
  t();
  assert.equal(await (await p).text(), "lazy()", "the cache answers once the timeout passes");
  assert.equal(w.netCount("static/lazy.js"), 0);
}
{ // old caches go, the current ones stay; an earlier app cache stays until this version's fill is done
  const w = mondo();
  for (const n of ["mzlab-app-old", "mzlab-app-abc", "mzlab-big-v-1", "mzlab-big-v-0", "qqq-app-x", "other"]) await w.ctx.caches.open(n);
  await new Promise(r => { w.listeners.activate({ waitUntil: p => p.then(r) }); });
  assert.deepEqual((await w.ctx.caches.keys()).sort(), ["mzlab-app-abc", "mzlab-app-old", "mzlab-big-v-1", "other"]);
}
{ // after a deploy: a file with the same bytes comes from the earlier version's cache, a changed one from the network
  const w = mondo();
  const old = await w.ctx.caches.open("mzlab-app-old");
  await old.put(new Request(O + "static/app.js"), new Response("app()", { status: 200 }));
  await old.put(new Request(O + "static/lazy.js"), new Response("lazy() of v0", { status: 200 }));
  assert.equal(await (await w.get("static/app.js")).text(), "app()");
  assert.equal(w.netCount("static/app.js"), 0, "unchanged file: not downloaded again " + w.st.fetches);
  assert.ok(await (await w.ctx.caches.open("mzlab-app-abc")).match(O + "static/app.js"), "copied into this version's cache");
  assert.equal(await (await w.get("static/lazy.js")).text(), "lazy()", "changed file: the bytes of this version");
  assert.equal(w.netCount("static/lazy.js"), 1);
}
{ // the fill copies the unchanged files, downloads the others, then the earlier cache goes
  const w = mondo();
  const old = await w.ctx.caches.open("mzlab-app-old");
  for (const p of ["index.html", "static/app.js", "static/big.wasm"]) await old.put(new Request(O + p), new Response(FILES[p], { status: 200 }));
  w.listeners.install(); w.timers[0]();
  const cache = await w.ctx.caches.open("mzlab-app-abc");
  for (let i = 0; i < 200 && !await cache.match(O + ".fill-done"); i++) await new Promise(r => setTimeout(r, 10));
  for (let i = 0; i < 200 && (await w.ctx.caches.keys()).includes("mzlab-app-old"); i++) await new Promise(r => setTimeout(r, 10));
  for (const p of ["index.html", "static/app.js", "static/big.wasm"]) assert.equal(w.netCount(p), 0, p + " copied, not downloaded");
  assert.equal(w.netCount("static/lazy.js"), 1);
  assert.ok(await cache.match(O + "static/big.wasm"));
  assert.deepEqual((await w.ctx.caches.keys()).filter(n => n.startsWith("mzlab-app-")), ["mzlab-app-abc"], "earlier cache deleted once full");
}
{ // a worker of an older version (the manifest on the network is newer) never deletes the other app caches
  const w = mondo(undefined, manifest({ app: "newer" }));
  const mine = await w.ctx.caches.open("mzlab-app-abc");
  await mine.put(new Request(O + ".fill-done"), new Response("ok"));
  await w.ctx.caches.open("mzlab-app-newer");
  w.listeners.install(); w.timers[0]();
  await new Promise(r => setTimeout(r, 50));
  assert.ok((await w.ctx.caches.keys()).includes("mzlab-app-newer"), "the newer version's cache stays");
}
{ // install does not wait; the background fill stores every file of the manifest except the big ones, then leaves its mark
  const w = mondo();
  w.listeners.install();
  assert.equal(w.st.fetches.length, 0, "nothing fetched at install time");
  w.timers[0]();
  const cache = await w.ctx.caches.open("mzlab-app-abc");
  for (let i = 0; i < 200 && !await cache.match(O + ".fill-done"); i++) await new Promise(r => setTimeout(r, 10));
  for (const p of ["index.html", "static/app.js", "static/lazy.js", "static/big.wasm"]) assert.ok(await cache.match(O + p), p);
  assert.ok(await cache.match(O), "the page as its folder");
  assert.equal(await cache.match(O + "static/esempi/a.mzML"), undefined, "nofill");
  assert.ok(await cache.match(O + ".fill-done"));
  const n = w.st.fetches.length;
  w.timers[0](); await new Promise(r => setTimeout(r, 50));
  assert.equal(w.st.fetches.length, n, "marked: no second fill");
}
console.log("sw_cache_node ok");
