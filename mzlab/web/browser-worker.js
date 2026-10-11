// Web Worker of the browser version: Pyodide (Python + numpy in WebAssembly) running the same mzlab package as the local program.
import { loadPyodide } from "./pyodide/pyodide.mjs";
import * as ARCH from "./archivio.js";
import { createPool, poolSize } from "./idxpool.js";

// a step of the start: a catalog key (shown by the loading screen in the language of the page) and the percentage when known
const say = (key, pct = null, params) => postMessage({ type: "step", key, pct, params });
const WORK = "/work/sessione";
const Q = new URL(import.meta.url).searchParams;
const STALL = +Q.get("r") || 30000;     // ms without any progress while reopening the saved files (?ripresa=ms on the page, for the tests)
const FRESH = Q.get("riparti") === "1"; // «start from scratch»: the saved files and the notebook are deleted
const OPFS = ARCH.available() && Q.get("opfs") !== "0";   // files and indexes in OPFS (?opfs=0 on the page keeps the old way: memory + IndexedDB, for the tests)

// ---- the browser's own storage (IndexedDB): files and notebook survive a reload, like the work folder of the local program
const DB = "qqq_lab"; // kept from the old name: renaming it would lose the users' data
let dbP = null;      // one connection for the whole life of the worker; it closes itself when another tab needs to upgrade the database
const idb = () => dbP || (dbP = new Promise((res, rej) => {
  const r = indexedDB.open(DB, 2);
  r.onupgradeneeded = () => {
    const db = r.result;
    if (!db.objectStoreNames.contains("files")) db.createObjectStore("files");
    if (!db.objectStoreNames.contains("kv")) db.createObjectStore("kv");
    if (!db.objectStoreNames.contains("liste_utente")) db.createObjectStore("liste_utente", { keyPath: "id" });
  };
  r.onblocked = () => postMessage({ type: "blocked" });     // another tab holds an older connection: the upgrade waits until it is closed
  r.onsuccess = () => { const db = r.result; db.onversionchange = () => { db.close(); dbP = null; }; res(db); };
  r.onerror = () => rej(r.error);
}).catch(e => { dbP = null; throw e; }));
const tx = async (store, mode, fn) => {
  const db = await idb();
  return new Promise((res, rej) => { const t = db.transaction(store, mode), s = t.objectStore(store), out = fn(s); t.oncomplete = () => res(out && out.result !== undefined ? out.result : undefined); t.onerror = t.onabort = () => rej(t.error); });
};
const safe = async f => { try { return await f(); } catch (_) { return undefined; } };   // private windows may refuse storage: the program still works, it just does not resume

// Big files are not copied into the memory of the engine: the Blob is mounted read-only with WORKERFS (/big/N/data) and linked into the work folder.
// With OPFS every mzML goes this way (a File from OPFS, mounted); without it only the files above BIG do, and above KEEP they are not saved in IndexedDB either
// (they have to be opened again after a reload).
const BIG = 50 << 20, KEEP = 300 << 20, MAX = 4 * 1024 * 1024 * 1024;
let bigN = 0;
const baseName = n => String(n || "").split(/[\\/]/).pop();
const mounts = new Map();        // "files/<name>" or "idx/<name>" -> the directory it is mounted at
function mountBlob(blob, key, root = "/big") {
  unmount(key);
  const dir = `${root}/${++bigN}`;
  py.FS.mkdirTree(dir);
  py.FS.mount(py.FS.filesystems.WORKERFS, { blobs: [{ name: "data", data: blob }] }, dir);
  if (key) mounts.set(key, dir);
  return `${dir}/data`;
}
const mountBig = (blob, key) => mountBlob(blob, key);
function unmount(key) {
  const dir = key && mounts.get(key);
  if (!dir) return;
  try { py.FS.unmount(dir); } catch (_) { /* busy: left mounted */ }
  mounts.delete(key);
}

let py = null, handle = null, linkBig = null, br = null;
// Everything the engine needs is requested at once, in parallel: before, numpy waited for Python to be compiled and the program
// waited for numpy (about 3 s lost on a 20 Mbit/s line). The numpy wheel is only fetched here to warm the HTTP cache (and the
// service worker): loadPackage then finds it there instead of starting the download late.
const IDX = new URL("./pyodide/", import.meta.url).href;
const zipP = fetch(new URL("./mzlab.zip", import.meta.url)).then(r => r.arrayBuffer());
zipP.catch(() => {});
fetch(IDX + "pyodide-lock.json").then(r => r.json()).then(l => fetch(IDX + l.packages.numpy.file_name)).then(r => r.arrayBuffer()).catch(() => {});
// rejects when nothing moved for STALL ms: a database that never answers must not keep the loading screen forever
function stallGuard() {
  let t, rej; const p = new Promise((_, r) => { rej = r; });
  const tick = () => { clearTimeout(t); t = setTimeout(() => rej(Object.assign(new Error("reopening the saved files timed out"), { key: "load.err.resumeTimeout" })), STALL); };
  tick(); p.catch(() => {}); return { p, tick, stop: () => clearTimeout(t) };
}
const within = (p, ms) => Promise.race([p, new Promise(r => setTimeout(r, ms))]);
// The files and indexes in OPFS: every mzML is mounted (WORKERFS) and linked into the work folder; every index that fits goes in the registry of Python.
// What an earlier visit left in IndexedDB (files below 300 MB) is moved to OPFS first; a file that cannot be moved stays where it is and is opened as before.
async function resumeOpfs(tick, names, bigs) {
  const moved = [];
  for (const n of names || []) {
    const buf = await safe(() => tx("files", "readonly", s => s.get(n)));
    tick();
    if (!buf) continue;
    try { await ARCH.put("files", n, buf instanceof Blob ? buf : new Uint8Array(buf)); moved.push(n); }
    catch (e) { console.debug("[mzLab] not moved to OPFS", n, e); if (buf instanceof Blob) bigs.push([n, buf]); else py.FS.writeFile(`${WORK}/${n}`, new Uint8Array(buf)); }
  }
  for (const n of moved) await safe(() => tx("files", "readwrite", s => s.delete(n)));
  const have = new Set();
  for (const e of await ARCH.list("files")) {
    tick();
    const f = await ARCH.get("files", e.name);
    if (!f) continue;
    try { py.FS.symlink(mountBlob(f, "files/" + e.name), `${WORK}/${e.name}`); have.add(e.name); } catch (er) { console.debug("[mzLab] file not reopened", e.name, er); }
  }
  const reg = {};
  for (const e of await ARCH.list("idx")) {
    if (/\.part$/.test(e.name)) { await ARCH.remove("idx", e.name); continue; }       // an index a closed page left half written
    const name = e.name.replace(/\.mzidx$/, "");
    if (!have.has(name)) continue;                    // the index of a file that was taken away stays in the archive until the space is needed
    const f = await ARCH.get("idx", e.name);
    if (f) try { reg[name] = mountBlob(f, "idx/" + name, "/idx"); } catch (er) { console.debug("[mzLab] index not mounted", e.name, er); }
  }
  return reg;
}
async function resume(tick) {
  say("load.step.resume", 60);
  if (FRESH) {
    await within(safe(() => tx("files", "readwrite", s => s.clear())), 5000); await within(safe(() => tx("kv", "readwrite", s => s.clear())), 5000);
    if (OPFS) await within(safe(async () => { await ARCH.clear("files"); await ARCH.clear("idx"); }), 5000);
  }
  const names = FRESH ? [] : await safe(() => tx("files", "readonly", s => s.getAllKeys()));
  tick(); say("load.step.resume", 62);
  const bigs = [];
  let reg = {};
  if (OPFS) reg = (await safe(() => resumeOpfs(tick, names, bigs))) || {};
  else {
    let i = 0;
    for (const n of names || []) {
      const buf = await safe(() => tx("files", "readonly", s => s.get(n)));
      tick(); say("load.step.resume", 62 + 36 * (++i / names.length));
      if (!buf) continue;
      if (typeof Blob !== "undefined" && buf instanceof Blob) bigs.push([n, buf]);        // a big file: mounted below, once Python is up
      else py.FS.writeFile(`${WORK}/${n}`, new Uint8Array(buf));
    }
  }
  const nb = FRESH ? null : await safe(() => tx("kv", "readonly", s => s.get("notebook")));
  if (nb) py.FS.writeFile(`${WORK}/taccuino.json`, nb);
  for (const [n, blob] of bigs) { try { py.FS.symlink(mountBig(blob, "files/" + n), `${WORK}/${n}`); } catch (e) { console.debug("[mzLab] big file not reopened", n, e); } }
  return reg;
}
// a tablet or a phone keeps fewer blocks of the indexes in memory
const SMALL = (() => { try { return /iPad|iPhone|Android/i.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1); } catch (_) { return false; } })();
async function start() {
  try {
    say("load.step.python", 5);
    py = await loadPyodide({ indexURL: IDX });
    say("load.step.numpy", 30);
    await py.loadPackage("numpy");
    say("load.step.program", 50);
    const zip = await zipP;
    py.FS.mkdirTree("/qqq");
    py.unpackArchive(zip, "zip", { extractDir: "/qqq" });
    py.FS.mkdirTree(WORK);
    py.FS.mkdirTree("/idxtmp");
    const g = stallGuard();
    let reg = {};
    try { reg = await Promise.race([resume(g.tick), g.p]); } finally { g.stop(); }
    say("load.step.start", 99);
    py.runPython("import sys; sys.path.insert(0, '/qqq')\nfrom mzlab import browser");
    if (Object.keys(reg).length) py.globals.get("browser").register_indexes(py.toPy(reg));
    py.runPython("browser.start()");
    handle = py.runPython("browser.handle");
    linkBig = py.runPython("browser.link_big");
    br = py.globals.get("browser");
    br.configure(SMALL);
    postMessage({ type: "ready" });
  } catch (e) { postMessage({ type: "fatal", text: String(e && e.message || e), key: e && e.key }); }
}
const started = start();

// --- the archive (OPFS): space, indexes
let persistAsked = false;
async function persistOnce() {              // asked at the first save, not at the first visit; a refusal is not worth a message: the indexes are rebuilt by themselves
  if (persistAsked) return; persistAsked = true;
  try { if (navigator.storage && navigator.storage.persist && !(await navigator.storage.persisted())) await navigator.storage.persist(); } catch (_) { /* optional */ }
}
async function spaceInfo() {
  if (!OPFS) return { opfs: false };
  const files = await safe(() => ARCH.list("files")) || [], idx = await safe(() => ARCH.list("idx")) || [];
  const names = new Set(files.map(f => f.name));
  const gone = idx.filter(e => !names.has(e.name.replace(/\.mzidx$/, "")));
  const sum = l => l.reduce((a, e) => a + e.size, 0);
  const est = await ARCH.estimate();
  let persisted = false; try { persisted = !!(await navigator.storage.persisted()); } catch (_) { /* optional */ }
  return { opfs: true, files: sum(files), idx: sum(idx), idx_removed: sum(gone), n_removed: gone.length, usage: est.usage, quota: est.quota, persisted };
}
// the indexes of the files that were taken away go first; the index of a file still there is never deleted from under it (it is rebuilt by itself if lost)
async function freeSpace() {
  const files = new Set((await safe(() => ARCH.list("files")) || []).map(f => f.name));
  for (const e of await safe(() => ARCH.list("idx")) || []) {
    const name = e.name.replace(/\.mzidx$/, "");
    if (files.has(name)) continue;
    br && br.forget_index(name); unmount("idx/" + name);
    await ARCH.remove("idx", e.name);
  }
}
async function lowSpace() { const e = await ARCH.estimate(); return e.quota > 0 && e.usage / e.quota > 0.8; }

// the indexes Python built in memory are moved to OPFS and mounted; the open files switch to the mounted copy, the memory of the temporary one is freed
async function settle() {
  if (!OPFS || !br) return;
  let list;
  try { const r = br.new_indexes(); list = r.toJs(); r.destroy(); } catch (_) { return; }
  for (const [name, path] of list) {
    try {
      if (await lowSpace()) await freeSpace();
      const iname = name + ".mzidx";
      const f = await ARCH.putFromFS("idx", iname, py.FS, path);
      const ok = br.adopt_index(name, mountBlob(f, "idx/" + name, "/idx") );
      if (!ok) { unmount("idx/" + name); await ARCH.remove("idx", iname); }
    } catch (e) { console.debug("[mzLab] index kept in memory", name, e); }
  }
}
async function buildIndex(name) {
  if (!OPFS || !br) return;
  try { if (br.build_index(name)) await settle(); } catch (_) { /* optional */ }
}

// ---- the indexes of the uploaded files are built by the pool of Rust workers (idxpool.js, idx-worker.js); Python builds them when the pool cannot (?pool=0 forces Python)
const POOLQ = Q.get("pool");
const pool = OPFS && POOLQ !== "0" ? createPool({ workerUrl: new URL("./idx-worker.js", import.meta.url), size: /^\d+$/.test(POOLQ || "") ? +POOLQ : poolSize(SMALL, navigator.hardwareConcurrency), budget: SMALL ? 32 : 64, notify: key => postMessage({ type: "notice", key }) }) : null;
const inChain = fn => { const p = chain.then(fn); chain = p.catch(() => {}); return p; };      // one thing at a time with the requests
// The index of an uploaded file: the pool of Rust workers builds it (the Python reader has already read the headers: index_inputs), Python when the pool cannot.
// Resolves when the index is in place (adopted, mounted) or nothing more will be done.
function buildFor(base, file, wait = false) {
  const t0 = performance.now();
  const note = (engine, why, passes) => postMessage({ type: "idx", name: base, engine, why, passes, ms: Math.round(performance.now() - t0) });
  const python = async why => { await inChain(() => buildIndex(base)); note("python", why); };
  return (async () => {
    if (!pool || pool.unavailable()) return python("off");
    const inputs = await inChain(() => { const r = br.index_inputs(base); if (!r) return null; const a = r.toJs(); r.destroy(); return a; });
    if (!inputs) return python("inputs");                            // already indexed, or the headers cannot be read: Python decides
    const part = base + ".mzidx.part", res = await pool.build({ name: base, blob: file, inputs, part });
    if (!res.ok) { await ARCH.remove("idx", part); if (res.reason !== "cancelled") await python(res.reason + ": " + (res.text || "")); return; }
    try {
      const f = await ARCH.promote("idx", part, base + ".mzidx");
      const ok = await inChain(async () => {
        if (await lowSpace()) await freeSpace();
        const adopted = br.adopt_index(base, mountBlob(f, "idx/" + base, "/idx"));
        if (!adopted) unmount("idx/" + base);
        return adopted;
      });
      if (!ok) { await ARCH.remove("idx", base + ".mzidx"); return python("adopt"); }
      note("rust", null, res.stats && res.stats.passes);
    } catch (e) { console.debug("[mzLab] index of the pool not used", base, e); await python(String(e && e.message || e)); }
  })().catch(e => console.debug("[mzLab] index", base, e));
  return p;
}

let chain = Promise.resolve();      // the requests are answered one after the other: an index moved to OPFS is never swapped under a request in the middle
onmessage = ev => { chain = chain.then(() => answer(ev.data)).catch(e => console.debug("[mzLab]", e)); };

async function answer({ id, method, url, body, blob }) {
  await started;
  if (!handle) return postMessage({ id, status: 500, text: JSON.stringify({ error: "calculation engine not started", error_key: "err.engine.notStarted" }) });
  let status, text, ctype, uploaded = null, later = null;
  const t0 = performance.now();
  const u = new URL(url, "http://x/");
  try {
    if (u.pathname === "/api/spazio") {                                              // storage of the browser: how much, and free what can go
      if (method === "POST") await freeSpace();
      status = 200; text = JSON.stringify(await spaceInfo());
    } else if (blob && (OPFS || blob.size > BIG)) {                                  // an upload: file in OPFS (or the Blob as it is), mounted, not copied
      const name = u.searchParams.get("name"), base = baseName(name);
      if (blob.size > MAX) throw Object.assign(new Error("file over 4 GB: too big"), { key: "err.memory.over4gb" });
      let file = blob;
      if (OPFS) {
        if (pool) pool.cancel(base);
        br.forget_index(base); unmount("idx/" + base); await ARCH.remove("idx", base + ".mzidx");      // a file of the same name, possibly other content: its index goes
        try { file = await ARCH.put("files", base, blob); persistOnce(); }
        catch (e) {                                                                      // no room (a private window has little): the indexes of removed files go, then one more try
          if (/quota/i.test(String(e && (e.name || e.message)))) { await freeSpace(); try { file = await ARCH.put("files", base, blob); } catch (e2) { e = e2; } }
          if (file === blob) console.debug("[mzLab] not saved in OPFS, used as it is for this visit", e);                // the Blob itself is mounted: it works until the page is closed, as before
        }
      }
      const target = mountBlob(file, "files/" + base);
      if (pool && OPFS && file !== blob) {                                              // the index first (pool), then the file is read: its answer comes when both are done, the requests in between are served
        later = (async () => {
          await inChain(() => { const x = linkBig(name, target, false); x.destroy(); });
          await buildFor(base, file);
          const r2 = await inChain(() => { const x = linkBig(name, target); const y = x.toJs(); x.destroy(); return y; });
          return r2;
        })();
      } else {
        const r = linkBig(name, target);
        [status, text] = r.toJs(); r.destroy();
      }
      if (status === 200 && file === blob && blob.size <= KEEP) { await safe(() => tx("files", "readwrite", s => s.put(blob, base))); persistOnce(); }
      if (status === 200 && OPFS && file !== blob) uploaded = base;
    } else {
      if (blob) { body = new Uint8Array(await blob.arrayBuffer()); blob = null; }       // a small file without OPFS: through Python, as before
      const r = handle(method, url, body);
      const res = r.toJs(); r.destroy(); status = res[0]; text = res[1]; ctype = res[2];
    }
  } catch (e) {
    const m = String(e && e.message || e);
    status = 500; text = JSON.stringify(e && e.key ? { error: m, error_key: e.key } : /memory|alloc|RangeError/i.test(m) ? { error: "this file is too big for the memory of the browser", error_key: "err.memory.tooBig" } : { error: m });
  }
  if (later) { later.then(([st, tx]) => postMessage({ id, status: st, text: tx, ms: performance.now() - t0 }), e => postMessage({ id, status: 500, text: JSON.stringify({ error: String(e && e.message || e) }) })); return; }
  if (text instanceof Uint8Array) postMessage({ id, status, buf: text, ctype, ms: performance.now() - t0 }, [text.buffer]);       // a binary block (api/scanbin): moved, not copied
  else postMessage({ id, status, text, ms: performance.now() - t0 });
  if (uploaded) { const t1 = performance.now(); await buildIndex(uploaded); postMessage({ type: "idx", name: uploaded, engine: "python", why: "off", ms: Math.round(performance.now() - t1) }); }       // right after the upload, before the next request: the first chart then finds the index
  if (method !== "POST" || status !== 200) { if (!uploaded) await settle(); return; }
  // keep the browser's storage in step with the program
  if (u.pathname === "/api/upload") { const n = u.searchParams.get("name"); const base = n && baseName(n); if (base && body && !blob) { await safe(() => tx("files", "readwrite", s => s.put(body.buffer.byteLength ? body.buffer : new ArrayBuffer(0), base))); persistOnce(); } }
  else if (u.pathname === "/api/remove") {
    try {
      const n = JSON.parse(new TextDecoder().decode(body)).name;
      if (n) {
        const base = n.split(/[\\/]/).pop();
        await safe(() => tx("files", "readwrite", s => s.delete(base)));
        if (pool) pool.cancel(base);
        if (OPFS) { unmount("files/" + base); await ARCH.remove("files", base); }        // its index stays (cache): the space dialog frees it first
      }
    } catch (_) { /* ignore */ }
  }
  else if (u.pathname === "/api/notebook") await safe(() => tx("kv", "readwrite", s => s.put(new TextDecoder().decode(body), "notebook")));
  else if (u.pathname === "/api/new") { try { if (JSON.parse(new TextDecoder().decode(body)).fresh) { await safe(() => tx("files", "readwrite", s => s.clear())); await safe(() => tx("kv", "readwrite", s => s.clear())); if (OPFS) { await ARCH.clear("files"); await ARCH.clear("idx"); } } } catch (_) { /* ignore */ } }
  if (!uploaded) await settle();
}
