// Web Worker of the browser version: Pyodide (Python + numpy in WebAssembly) running the same mzlab package as the local program.
import { loadPyodide } from "./pyodide/pyodide.mjs";

// a step of the start: a catalog key (shown by the loading screen in the language of the page) and the percentage when known
const say = (key, pct = null, params) => postMessage({ type: "step", key, pct, params });
const WORK = "/work/sessione";
const Q = new URL(import.meta.url).searchParams;
const STALL = +Q.get("r") || 30000;     // ms without any progress while reopening the saved files (?ripresa=ms on the page, for the tests)
const FRESH = Q.get("riparti") === "1"; // «start from scratch»: the saved files and the notebook are deleted

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
// Above KEEP the file is not saved in IndexedDB either (it has to be opened again after a reload).
const BIG = 50 << 20, KEEP = 300 << 20, MAX = 4 * 1024 * 1024 * 1024;
let bigN = 0;
const baseName = n => String(n || "").split(/[\\/]/).pop();
function mountBig(blob) {
  const dir = `/big/${++bigN}`;
  py.FS.mkdirTree(dir);
  py.FS.mount(py.FS.filesystems.WORKERFS, { blobs: [{ name: "data", data: blob }] }, dir);
  return `${dir}/data`;
}

let py = null, handle = null, linkBig = null;
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
async function resume(tick) {
  say("load.step.resume", 60);
  if (FRESH) { await within(safe(() => tx("files", "readwrite", s => s.clear())), 5000); await within(safe(() => tx("kv", "readwrite", s => s.clear())), 5000); }
  const names = FRESH ? [] : await safe(() => tx("files", "readonly", s => s.getAllKeys()));
  tick(); say("load.step.resume", 62);
  const bigs = [];
  let i = 0;
  for (const n of names || []) {
    const buf = await safe(() => tx("files", "readonly", s => s.get(n)));
    tick(); say("load.step.resume", 62 + 36 * (++i / names.length));
    if (!buf) continue;
    if (typeof Blob !== "undefined" && buf instanceof Blob) bigs.push([n, buf]);        // a big file: mounted below, once Python is up
    else py.FS.writeFile(`${WORK}/${n}`, new Uint8Array(buf));
  }
  const nb = FRESH ? null : await safe(() => tx("kv", "readonly", s => s.get("notebook")));
  if (nb) py.FS.writeFile(`${WORK}/taccuino.json`, nb);
  for (const [n, blob] of bigs) { try { py.FS.symlink(mountBig(blob), `${WORK}/${n}`); } catch (e) { console.debug("[mzLab] big file not reopened", n, e); } }
}
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
    const g = stallGuard();
    try { await Promise.race([resume(g.tick), g.p]); } finally { g.stop(); }
    say("load.step.start", 99);
    py.runPython("import sys; sys.path.insert(0, '/qqq')\nfrom mzlab import browser\nbrowser.start()");
    handle = py.runPython("browser.handle");
    linkBig = py.runPython("browser.link_big");
    postMessage({ type: "ready" });
  } catch (e) { postMessage({ type: "fatal", text: String(e && e.message || e), key: e && e.key }); }
}
const started = start();

onmessage = async ev => {
  const { id, method, url, body, blob } = ev.data;
  await started;
  if (!handle) return postMessage({ id, status: 500, text: JSON.stringify({ error: "calculation engine not started", error_key: "err.engine.notStarted" }) });
  let status, text, ctype;
  const t0 = performance.now();
  try {
    if (blob) {                                                                       // a big upload: mount the Blob, do not copy it
      const name = new URL(url, "http://x/").searchParams.get("name");
      if (blob.size > MAX) throw Object.assign(new Error("file over 4 GB: too big"), { key: "err.memory.over4gb" });
      const r = linkBig(name, mountBig(blob));
      [status, text] = r.toJs(); r.destroy();
      if (status === 200 && blob.size <= KEEP) await safe(() => tx("files", "readwrite", s => s.put(blob, baseName(name))));
    } else {
      const r = handle(method, url, body);
      const res = r.toJs(); r.destroy(); status = res[0]; text = res[1]; ctype = res[2];
    }
  } catch (e) {
    const m = String(e && e.message || e);
    status = 500; text = JSON.stringify(e && e.key ? { error: m, error_key: e.key } : /memory|alloc|RangeError/i.test(m) ? { error: "this file is too big for the memory of the browser", error_key: "err.memory.tooBig" } : { error: m });
  }
  if (text instanceof Uint8Array) postMessage({ id, status, buf: text, ctype, ms: performance.now() - t0 }, [text.buffer]);       // a binary block (api/scanbin): moved, not copied
  else postMessage({ id, status, text, ms: performance.now() - t0 });
  if (method !== "POST" || status !== 200) return;
  // keep the browser's storage in step with the program
  const u = new URL(url, "http://x/");
  if (u.pathname === "/api/upload") { const n = u.searchParams.get("name"); const base = n && baseName(n); if (base && body) await safe(() => tx("files", "readwrite", s => s.put(body.buffer.byteLength ? body.buffer : new ArrayBuffer(0), base))); }
  else if (u.pathname === "/api/remove") { try { const n = JSON.parse(new TextDecoder().decode(body)).name; if (n) await safe(() => tx("files", "readwrite", s => s.delete(n.split(/[\\/]/).pop()))); } catch (_) { /* ignore */ } }
  else if (u.pathname === "/api/notebook") await safe(() => tx("kv", "readwrite", s => s.put(new TextDecoder().decode(body), "notebook")));
  else if (u.pathname === "/api/new") { try { if (JSON.parse(new TextDecoder().decode(body)).fresh) { await safe(() => tx("files", "readwrite", s => s.clear())); await safe(() => tx("kv", "readwrite", s => s.clear())); } } catch (_) { /* ignore */ } }
};
