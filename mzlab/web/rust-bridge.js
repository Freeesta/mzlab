// Page side of the Rust engine (?motore=rust or localStorage qqq.motore = "rust"): browser.js hands every GET api/chrom (TIC), api/xic and
// api/spectra to this module first. It answers them from rust-worker.js when it can answer exactly as Python does (same JSON); otherwise
// it returns null and Pyodide answers. Every other route always goes to Pyodide. Without the option this file is never loaded.
//
// Supervisor: if the worker dies (a trap, a lost memory, a script error) it is terminated, a new one is started, the files of the session
// are opened again and the page shows `error.engineRestarted`; a second death within a minute switches Rust off for the rest of the session
// (everything goes to Pyodide, `error.engineFallback`).
export function createBridge({ workerUrl, notify }) {
  const blobs = new Map();                 // file name -> Blob of every file uploaded in this page
  let slots = [];                          // k -> { name, blob, info, bad, opening }
  let worker = null, seq = 0, restarts = [];
  const pending = new Map();               // id -> { res, worker }
  const stats = { rust: 0, python: 0, restarts: 0 };
  const api = { enabled: true, disabled: false, stats };
  const base = n => String(n || "").split(/[\\/]/).pop();

  function spawn() {
    const w = new Worker(workerUrl, { type: "module" });
    w.onmessage = ev => {
      const m = ev.data;
      if (m.op === "ready") return;
      if (m.op === "progress") { const q = pending.get(m.id); if (q && q.onp) q.onp(m); return; }       // a conversion that is still running
      const p = pending.get(m.id);
      if (!p) { if (m.buf) recycle(m.buf); return; }
      pending.delete(m.id);
      if (m.fatal) { p.res({ dead: true, error: m.error }); died(w); } else p.res(m);
    };
    w.onerror = () => died(w);
    worker = w;
  }
  function recycle(buf) { if (worker) worker.postMessage({ op: "recycle", bufs: [buf] }, [buf]); }
  function rpc(msg, onp) {
    return new Promise(res => {
      if (!worker || api.disabled) return res({ dead: true });
      const id = ++seq;
      pending.set(id, { res, worker, onp });
      worker.postMessage({ ...msg, id });
    });
  }
  function died(w) {
    if (w !== worker) return;
    worker.terminate(); worker = null;
    for (const [id, p] of pending) if (p.worker === w) { pending.delete(id); p.res({ dead: true }); }
    const now = Date.now();
    restarts = restarts.filter(t => now - t < 60000); restarts.push(now); stats.restarts++;
    if (restarts.length > 1) { api.disabled = true; slots.forEach(s => { if (s) s.bad = true; }); notify("error.engineFallback"); return; }
    spawn();
    slots.forEach((s, k) => { if (s) { s.bad = false; s.opening = open(k, s); } });       // the files come back from the session
    notify("error.engineRestarted");
  }
  function open(k, s) {
    return rpc({ op: "open", slot: k, blob: s.blob }).then(r => {
      if (r.dead || r.error) {
        s.bad = true;
        let key = ""; try { key = JSON.parse(r.error).error_key; } catch (_) { /* not an engine error */ }
        if (key === "error.memoryBudget") notify("error.memoryBudget");           // this file is left to Python
        return;
      }
      s.info = JSON.parse(r.info);
    });
  }

  api.start = () => { if (!worker) spawn(); };
  api.noteUpload = (name, body) => { try { blobs.set(base(name), body instanceof Blob ? body : new Blob([body])); } catch (_) { /* optional */ } };
  api.noteRemove = name => { blobs.delete(base(name)); };
  api.noteNew = () => { blobs.clear(); api.noteExplore(null); };
  // the session: samples[k].file is the file of item k; a part of a mixed file ("name.mzML#tag") is left to Python
  api.noteExplore = body => {
    slots = []; if (worker && !api.disabled) worker.postMessage({ op: "reset", id: ++seq });
    for (const [k, s] of ((body && body.samples) || []).entries()) {
      const f = String(s.file || ""), blob = blobs.get(base(f));
      if (!blob || f.includes("#") || api.disabled) { slots[k] = null; continue; }
      slots[k] = { name: base(f), blob, info: null, bad: false };
      slots[k].opening = open(k, slots[k]);
    }
  };
  // Thermo .raw -> Blob of mzML, made in the worker; rejects with an Error whose `json` is { error, error_key, params } like the API errors
  api.convertRaw = async (file, onProgress) => {
    const r = await rpc({ op: "raw2mzml", name: base(file.name), blob: file }, onProgress);
    if (r.dead || r.error) {
      let e = { error_key: "err.file.raw", params: { detail: "" } };
      try { const j = JSON.parse(r.error); if (j.error_key) e = j; } catch (_) { /* the worker was lost */ }
      throw Object.assign(new Error(e.error_key), { json: { error: e.error_key, ...e } });
    }
    return Object.assign(new Blob(r.parts, { type: "application/xml" }), { mzlabId: r.tid ?? null });       // tid: the number of the mzML on the disk (desktop app only)
  };
  api.crash =() => rpc({ op: "crash" });

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

  // Returns { status, text } or null (Pyodide answers).
  api.handle = async (method, url) => {
    if (!api.enabled || api.disabled || method !== "GET" || !worker) return null;
    const u = new URL(url, "http://x/"), q = u.searchParams, path = u.pathname;
    const level = num(q, "level", 1), px = Math.max(0, num(q, "px", 0) | 0);
    let ks;
    if (path === "/api/chrom") {
      if ((q.get("kind") || "tic") !== "tic" || !empty(q, "mz0", "mz1", "prec", "filt")) return null;
      ks = [num(q, "k", -1)];
    } else if (path === "/api/xic") ks = (q.get("k") || "").split(",").filter(Boolean).map(Number);
    else if (path === "/api/spectra") {
      if (!empty(q, "prec", "precursor", "filt") || q.get("merge") === "1") return null;
      ks = [num(q, "k", -1)];
    } else return null;
    if (!ks.length || ks.some(k => !Number.isInteger(k) || !slots[k])) return null;
    await Promise.all(ks.map(k => slots[k].opening));
    if (ks.some(k => !slots[k] || slots[k].bad || !slots[k].info || !slots[k].info.spectra)) return null;
    const t0 = performance.now();
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
