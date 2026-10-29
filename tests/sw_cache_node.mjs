// Loads mzlab/web/sw.js with a mocked service-worker scope and counts cache.put calls: an app file revalidated with the same ETag is not rewritten.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const src = fs.readFileSync(path.join(root, "mzlab/web/sw.js"), "utf8");

/** Runs sw.js; returns fetchUrl(url, headers) for the app cache and the put counter. */
function mondo() {
  const listeners = {}, store = new Map(), state = { puts: 0, bytes: 0, net: null };
  const cache = {
    async match(req) { const r = store.get(req.url); return r ? r.clone() : undefined; },
    async put(req, res) { state.puts++; state.bytes += Number(res.headers.get("Content-Length") || 0); store.set(req.url, res); },
  };
  const sw = { addEventListener: (n, f) => { listeners[n] = f; }, skipWaiting() {}, clients: { claim() {} } };
  const ctx = {
    self: sw, location: { origin: "https://x.test" }, URL, Headers, Response, importScripts() {},
    caches: { open: async () => cache, keys: async () => [], delete: async () => true },
    fetch: async () => state.net(),
  };
  vm.runInNewContext(src, ctx);
  const fetchUrl = async (url, headers) => {
    state.net = () => new Response("body", { status: 200, headers });
    let p;
    listeners.fetch({ request: { method: "GET", url, headers: new Headers(), cache: "default", mode: "same-origin" }, respondWith: x => { p = x; } });
    const res = await p;
    assert.equal(res.headers.get("Cross-Origin-Embedder-Policy"), "require-corp");
    return res;
  };
  return { fetchUrl, state };
}
const u = "https://x.test/static/app.js";

{ const { fetchUrl, state } = mondo();
  await fetchUrl(u, { ETag: '"a"' }); await fetchUrl(u, { ETag: '"a"' });
  assert.equal(state.puts, 1, "same ETag: one put"); }
{ const { fetchUrl, state } = mondo();
  await fetchUrl(u, { ETag: '"a"' }); await fetchUrl(u, { ETag: '"b"' });
  assert.equal(state.puts, 2, "changed ETag: put again"); }
{ const { fetchUrl, state } = mondo();
  await fetchUrl(u, {}); await fetchUrl(u, {});
  assert.equal(state.puts, 2, "no validators: put every time"); }
{ const { fetchUrl, state } = mondo();
  const h = { "Last-Modified": "Mon, 01 Jan 2026 00:00:00 GMT", "Content-Length": "4" };
  await fetchUrl(u, h); await fetchUrl(u, h);
  assert.equal(state.puts, 1, "same Last-Modified+Content-Length: one put");
  await fetchUrl(u, { ...h, "Content-Length": "5" });
  assert.equal(state.puts, 2, "different length: put"); }
console.log("sw_cache_node ok");
