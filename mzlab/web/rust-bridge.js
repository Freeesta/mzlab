// @ts-strict
// Page side of the Rust engine: browser.js hands every GET of the routes in RUST_ROUTES to this module first. It answers them from
// rust-worker.js (the same JSON as mzlab/api.py, the reference of the tests); null = the request is not one it answers.
//
// Supervisor: if the worker dies (a trap, a lost memory, a script error) it is terminated, a new one is started, the files of the session
// are opened again and the page shows `error.engineRestarted`; a second death within a minute switches the engine off for the rest of the
// session (`error.engineFallback`: the page asks to be reloaded). A death with the signature of a lack of memory (a trap or `rust_oom` after the memory grew) is not
// retried as it is: once the engine restarts with half the budget, a second time the engine is off with `error.memoryBudget`, never a loop of restarts.
// The .wasm is compiled once here and the same WebAssembly.Module goes to every worker of the engine (null = the worker compiles it).
import { infoRoute } from "./rust-bridge-info.js";
import { hasScan, scanRoutes } from "./rust-scans.js";
import { diffRoute } from "./rust-diff.js";
import { makeConvertRaw } from "./rust-raw.js";
import { makeConvertCentroids } from "./rust-centroidi.js";
import { wiffApi } from "./rust-convert.js";
import { makeNativo } from "./rust-nativo-pagina.js";
import { makeRecycler } from "./rust-recycle.js";
import { compileOnce, ordered, judgeDeath, forgetFaults } from "./rust-supervisor.js";

export function createBridge({ workerUrl, notify, onBlocked = () => {}, nativo = false }) {
  const blobs = new Map();                 // file name -> Blob of every file uploaded in this page
  let slots = [];                          // k -> { name, blob, info, bad, opening }
  let worker = null, seq = 0, peakMem = 0;
  const pending = new Map();               // id -> { res, worker }
  const stats = { rust: 0, python: 0, restarts: 0, mzml: 0 };       // mzml: how many times a file was read as an mzML (not from its index)
  const api = { enabled: true, disabled: false, stats, budgetMb: 0, halved: false, lastPanic: /** @type {string | null} */ (null) };
  const death = { restarts: /** @type {number[]} */ ([]), api, stats };       // for judgeDeath
  const base = n => String(n || "").split(/[\\/]/).pop() || "";
  const recycler = api.recycler = makeRecycler({ pending, worker: () => worker, recycleNow: () => { stop(worker, { dead: true }); stats.recycled = (stats.recycled || 0) + 1; respawn(); }, disabled: () => api.disabled });
  const wasmModule = compileOnce(new URL("wasm/mzlab_wasm_bg.wasm", workerUrl));
  function spawn() {
    const w = /** @type {Worker & { send?: any }} */ (new Worker(workerUrl, { type: "module" })); peakMem = 0; w.send = ordered(w, wasmModule, () => api.budgetMb, () => ++seq);
    w.onmessage = ev => {
      const m = ev.data;
      if (m.op === "ready") return;
      if (m.op === "blocked") { onBlocked(); return; }                                                     // the saved files are held by another tab
      if (m.op === "notice") { notify(m.key); return; }                                                    // e.g. error.memoryBudget from the pool of the indexes
      if (m.op === "progress") { const q = pending.get(m.id); if (q && q.onp) q.onp(m); return; }       // a conversion that is still running
      const p = pending.get(m.id);
      if (!p) { if (m.buf) recycle(m.buf); return; }
      pending.delete(m.id); peakMem = Math.max(peakMem, m.mem || 0); recycler.seen(m.mem);
      if (m.fatal) { p.res({ dead: true, error: m.error }); died(w, { text: m.error, panic: m.panic, mem: m.mem }); } else p.res(m);
    };
    w.onerror = e => died(w, { text: e && e.message });
    worker = w;
  }
  function recycle(buf) { if (worker) worker.send({ op: "recycle", bufs: [buf] }, [buf]); }
  function rpc(msg, onp) {
    return new Promise(res => {
      if (!worker || api.disabled) return res({ dead: true });
      const id = ++seq;
      pending.set(id, { res, worker, onp });
      worker.send({ ...msg, id });
    });
  }
  function stop(w, res) {
    worker.terminate(); worker = null;
    for (const [id, p] of pending) if (p.worker === w) { pending.delete(id); p.res(res); }
  }
  function respawn() {
    spawn();
    slots.forEach((s, k) => { if (s) { s.bad = false; s.idx = false; s.mzml = null; s.opening = open(k, s); } });       // the files come back from the session
  }
  function died(w, info) {
    if (w !== worker) return;
    stop(w, { dead: true });
    const act = judgeDeath(death, info, peakMem);
    if (act === "restart" || act === "halve") { respawn(); notify("error.engineRestarted"); return; }
    api.disabled = true; slots.forEach(s => { if (s) s.bad = true; }); notify(act === "oom" ? "error.memoryBudget" : "error.engineFallback");
  }
  // A file has ONE reader: its .mzidx (OPFS), which answers TIC, XIC, scan lists and filters without reading the mzML. The mzML itself is opened
  // only for what needs the spectra (single scans), the first time they are asked, or at once when the file has no index (no OPFS).
  function openMzml(k, s) {
    stats.mzml++; (stats.mzmlOf = stats.mzmlOf || []).push(s.name);
    if (!s.blob) { s.bad = true; return Promise.resolve(); }
    return rpc({ op: "open", slot: k, blob: s.blob }).then(r => {
      if (r.dead || r.error) {
        s.bad = true;
        let key = ""; try { key = JSON.parse(r.error).error_key; } catch (_) { /* not an engine error */ }
        if (key === "error.memoryBudget") notify("error.memoryBudget");           // this file is not opened (the page tells the user)
        return;
      }
      s.info = JSON.parse(r.info);
    });
  }
  const needMzml = (k, s) => s.mzml || (s.mzml = s.info ? Promise.resolve() : openMzml(k, s));
  function open(k, s) {
    return rpc({ op: "idxopen", slot: k, name: s.name, pol: s.pol }).then(r => {
      s.idx = r.dead || r.error ? null : JSON.parse(r.info);                 // { scans, hr1 }
      if (!s.idx) { s.mzml = openMzml(k, s); return s.mzml; }
    });
  }
  api.start = () => { if (!worker && !api.disabled) spawn(); };
  api.noteUpload = (name, body) => { try { blobs.set(base(name), body instanceof Blob ? body : new Blob([body])); } catch (_) { /* optional */ } };
  api.noteRemove = name => { blobs.delete(base(name)); if (api.unmarkNative) api.unmarkNative(name); };
  api.noteNew = () => { blobs.clear(); api.noteExplore(null); };
  // the session: samples[k].file is the file of item k; a part of a mixed file ("name.mzML#tag") shares the index of its file, the MRM part and a polarity part have their own slot on the same file (a polarity part reads the sections of its polarity)
  api.noteExplore = body => {
    const more = !!(body && body.append);                           // a file added to the session that is open: the others stay where they are
    if (!more) { slots = []; if (worker && !api.disabled) worker.send({ op: "reset", id: ++seq }); }
    const first = more ? slots.length : 0;
    for (const [i, s] of ((body && body.samples) || []).entries()) {
      const k = first + i;
      const f = String(s.file || ""), blob = blobs.get(base(f));
      const tag = f.includes("#") ? f.slice(f.indexOf("#") + 1) : "";
      if (api.disabled) { slots[k] = null; continue; }
      slots[k] = { name: base(f.split("#")[0]), blob: blobs.get(base(f.split("#")[0])) || blob || null, info: null, bad: false, idx: false, mzml: null, pol: /[ _]pos$/i.test(tag) ? 1 : /[ _]neg$/i.test(tag) ? -1 : 0 };
      slots[k].opening = open(k, slots[k]);
    }
  };
  // a worker busy in one long call cannot hear a message: cancelling stops it and starts a new one (not a fault). nativo (browser): the files of a maker are read as they are (rust-nativo-pagina.js); the desktop app converts a .raw (rust-raw.js)
  const cancelable = { rpc, base, current: () => worker, abort: (/** @type {any} */ w) => { stop(w, { dead: true, cancelled: true }); if (!api.disabled) respawn(); } };
  Object.assign(api, wiffApi(rpc, base), { convertCentroids: makeConvertCentroids(cancelable) }, nativo ? makeNativo(cancelable) : { convertRaw: makeConvertRaw(cancelable) });
  // true when the files at these slots have their index open (a part of a mixed file or a file without index: false)
  api.ready = async ks => {
    if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return false;
    await Promise.all(ks.map(k => slots[k].opening));
    return ks.every(k => slots[k] && slots[k].idx && !slots[k].bad);
  };
  api.crash = grow => rpc({ op: "crash", grow: grow || 0 });
  api.forgetFaults = () => { forgetFaults(death); if (!worker) respawn(); };      // test hook: starts the count of deaths again and brings a stopped engine back
  api.stop = () => { if (worker) { worker.terminate(); worker = null; } };                // reset.js: the worker holds sync handles on the files
  api.rpc = rpc;                                                       // for lr.js (the engine of the page)

  const num = (q, n, d) => { const v = q.get(n); return v === null || v === "" ? d : Number(v); };
  const empty = (q, ...names) => names.every(n => !q.get(n));
  const trace = async (k, op, level, extra, px) => {
    const r = await rpc({ op, slot: k, level, px, ...extra });
    if (r.dead || r.error) return null;
    const a = new Float64Array(r.buf, 0, r.len), n = r.n;
    const out = { rt: Array.from(a.subarray(0, n)), y: Array.from(a.subarray(n, 2 * n)) };
    recycle(r.buf);
    return out;
  };

  async function indexRoute(s, k, path, u, t0) {
    let r = await rpc({ op: "route", slot: k, path, query: u.search });
    if (!r.dead && !r.error && !r.info && path === "/api/spectrum" && s.blob) r = await rpc({ op: "specexact", slot: k, query: u.search, blob: s.blob });   // one scan of a centroid HR file: exact peaks from the mzML
    if (r.dead || r.error || !r.info) return null;
    stats.rust++;
    return { status: 200, text: r.info, ms: performance.now() - t0 };
  }
  const INDEX_ROUTES = /^\/api\/(chrom|filters|scanlist|nearest_scan|spectrum|dda|scan_axis)$/;
  const hasRef = q => !!q.get("ref");
  // Returns { status, text } or null (the route is unavailable: browser.js answers err.route.unavailable).
  api.handle = async (method, url) => {
    if (!api.enabled || api.disabled || method !== "GET" || !worker) return null;
    const u = new URL(url, "http://x/"), q = u.searchParams, path = u.pathname;
    const level = num(q, "level", 1), px = Math.max(0, num(q, "px", 0) | 0);
    const t0 = performance.now();
    const k0 = num(q, "k", -1);
    if (hasScan(path, q) && slots[k0]) {
      const r = await scanRoutes({ slots, rpc, stats }, k0, path, q, u.search);
      if (r) return { status: 200, ...r, ms: performance.now() - t0 };
    }
    if (hasRef(q) && /^\/api\/(chrom|xic|mrm)$/.test(path)) {
      const r = await diffRoute({ slots, rpc }, path, u);
      if (r) { stats.rust++; return { ...r, ms: performance.now() - t0 }; }
      return null;
    }
    if (INDEX_ROUTES.test(path) && !hasRef(q)) {
      const s = slots[k0];
      if (!s) return null;
      await s.opening;
      if (s.bad) return null;
      if (s.idx) return path === "/api/chrom" && !s.idx.scans ? infoRoute(path, u, { slots, rpc, base, stats, t0 }) : indexRoute(s, k0, path, u, t0);   // an MRM file has no scans: its chromatograms; a file without index goes on to the TIC/XIC/spectra below
    }
    if (path === "/api/mrm" && !hasRef(q)) {                         // the SRM transitions: chromatograms of the mzML (no scans, no peaks)
      const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
      if (!ks.length || !ks.every(k => Number.isInteger(k) && slots[k])) return null;
      await Promise.all(ks.map(k => slots[k].opening));
      if (!ks.every(k => slots[k] && slots[k].idx && !slots[k].bad && slots[k].blob)) return null;
      const parts = [];
      for (const k of ks) {
        const r = await rpc({ op: "mrm", slot: k, blob: slots[k].blob });
        if (r.dead || r.error || !r.info) return null;
        parts.push(`{"k":${k},"transitions":${r.info}}`);
      }
      stats.rust++;
      return { status: 200, text: `{"files":[${parts.join(",")}]}`, ms: performance.now() - t0 };
    }
    if (path === "/api/ms2tr" || path === "/api/fileinfo") return infoRoute(path, u, { slots, rpc, base, stats, t0 });
    if (path === "/api/scaninfo") {
      const s = slots[k0], sid = num(q, "sid", NaN);
      if (!s) return null;
      await s.opening;
      if (!s.idx || !s.idx.scans || s.bad || !s.blob || !Number.isInteger(sid) || sid < 0) return null;
      const r = await rpc({ op: "scaninfo", slot: k0, sid, blob: s.blob });
      if (r.dead || r.error || !r.info) return null;                         // an error (scan outside the file) is worded by Python
      stats.rust++;
      return { status: 200, text: r.info, ms: performance.now() - t0 };
    }
    if (path === "/api/xic" && !hasRef(q)) {
      const ks = (q.get("k") || "").split(",").filter(Boolean).map(Number), mz = num(q, "mz", NaN), tol = num(q, "tol", 0.35);
      if (ks.length && isFinite(mz) && ks.every(k => Number.isInteger(k) && slots[k])) {
        await Promise.all(ks.map(k => slots[k].opening));
        if (ks.every(k => slots[k] && slots[k].idx && !slots[k].bad)) {
          const parts = [];
          for (const k of ks) {
            const r = await rpc({ op: "idxxic", slot: k, level, mz, tol });
            if (r.dead || r.error || !r.info) return null;
            parts.push(`{"k":${k},${r.info.slice(1)}`);
          }
          stats.rust++;
          return { status: 200, text: `{"mz":${JSON.stringify(mz)},"tol":${JSON.stringify(tol)},"traces":[${parts.join(",")}]}`, ms: performance.now() - t0 };
        }
      }
    }
    // the rest needs the spectra of the mzML (or the file has no index): TIC and XIC of a file without index, single scans
    let ks;
    if (path === "/api/chrom") {
      if ((q.get("kind") || "tic") !== "tic" || !empty(q, "mz0", "mz1", "prec", "filt")) return null;
      ks = [k0];
    } else if (path === "/api/xic") ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
    else if (path === "/api/spectra") {
      if (!empty(q, "prec", "precursor", "filt") || q.get("merge") === "1") return null;
      ks = [k0];
    } else return null;
    if (!ks.length || ks.some(k => !Number.isInteger(k) || !slots[k])) return null;
    await Promise.all(ks.map(k => slots[k].opening));
    if (ks.some(k => slots[k].idx && !slots[k].idx.scans)) return null;             // an MRM file has no scans: Python's
    await Promise.all(ks.map(k => needMzml(k, slots[k])));
    if (ks.some(k => !slots[k] || slots[k].bad || !slots[k].info || !slots[k].info.spectra)) return null;
    let body;
    if (path === "/api/chrom") {
      const t = await trace(ks[0], "tic", level, {}, px); if (!t) return null; body = t;
    } else if (path === "/api/xic") {
      const mz = num(q, "mz", NaN), tol = num(q, "tol", 0.35);
      if (!isFinite(mz)) return null;
      const traces = [];
      for (const k of ks) { const t = await trace(k, "xic", level, { mz, tol }, px); if (!t) return null; traces.push({ k, ...t }); }
      body = { mz, tol, traces };
    } else {
      const info = slots[ks[0]].info, lv = info.levels[String(level)];
      const i0 = num(q, "i0", NaN), i1 = num(q, "i1", NaN);
      if (!info.unit || !lv || lv[1] || !(i0 >= 0 && i0 <= i1 && i1 - i0 + 1 <= 60 && i0 < lv[0])) return null;      // unit resolution centroids only
      const r = await rpc({ op: "spectra", slot: ks[0], level, i0, i1, bin: num(q, "bin", 0.1) });
      if (r.dead || r.error || !r.n || !r.buf) return null;
      const a = new Float64Array(r.buf, 0, r.len), scans = [];
      let w = 2;
      for (let c = 0; c < a[1]; c++) {
        const sid = a[w], rt = a[w + 1], n = a[w + 2]; w += 3;
        scans.push({ i: i0 + c, sid, rt, mode: "centroid", mz: Array.from(a.subarray(w, w + n)), y: Array.from(a.subarray(w + n, w + 2 * n)) });
        w += 2 * n;
      }
      body = { n: a[0], scans };
      recycle(r.buf);
    }
    stats.rust++;
    return { status: 200, text: JSON.stringify(body), ms: performance.now() - t0 };
  };
  return api;
}
