// Loads mzlab/web/sw.js with a mocked service-worker scope: app files are cache-first by site version (hit, miss -> network -> checked against
// the manifest -> put), navigations are network-first, a manifest of another version bypasses the cache, the background fill, old caches go.
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
  const listeners = {}, timers = [], stores = new Map(), st = { fetches: [], updates: 0, offline: false, net };
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
    setTimeout: (f, ms) => { timers.push(f); return timers.length; },
    caches: { open, keys: async () => [...stores.keys()], delete: async n => stores.delete(n) },
    fetch: async (r, init) => {
      const u = new URL(typeof r === "string" ? r : r.url);
      st.fetches.push(u.pathname.slice(1));
      if (st.offline) throw new TypeError("offline");
      if (u.pathname === "/static/sw-manifest.json") return new Response(man, { status: 200 });
      const body = st.net[u.pathname.slice(1) || "index.html"];
      return body === undefined ? new Response("", { status: 404 }) : new Response(body, { status: 200 });
    },
  };
  vm.runInNewContext(src, ctx);
  const get = async (p, extra = {}) => {
    let out; const e = { request: { method: "GET", url: O + p, headers: new Headers(), cache: "default", mode: "same-origin", ...extra }, respondWith: x => { out = x; }, waitUntil() {} };
    listeners.fetch(e);
    if (!out) return null;
    const res = await out;
    assert.equal(res.headers.get("Cross-Origin-Embedder-Policy"), "require-corp");
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
{ // the site on the network is another version: cache bypassed, the worker asks for an update
  const w = mondo({ ...FILES }, manifest({ app: "zzz" }));
  await w.get("static/app.js");
  assert.equal(w.st.updates, 1);
  await w.get("static/app.js");
  assert.equal(w.netCount("static/app.js"), 2, "stale: nothing stored, nothing served from the cache");
}
{ // old caches go, the current ones stay
  const w = mondo();
  for (const n of ["mzlab-app-old", "mzlab-app-abc", "mzlab-big-v-1", "mzlab-big-v-0", "qqq-app-x", "other"]) await w.ctx.caches.open(n);
  await new Promise(r => { w.listeners.activate({ waitUntil: p => p.then(r) }); });
  assert.deepEqual((await w.ctx.caches.keys()).sort(), ["mzlab-app-abc", "mzlab-big-v-1", "other"]);
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
