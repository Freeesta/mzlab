// Replaces static/rust-worker.js in the desktop app (prepare.py): the same messages as the WASM worker, but every answer comes from the native engine
// through the mzlab:// protocol (files are memory-mapped from the disk, no 150 MB limit for .raw). Nothing else answers: the operations of the page are
// `rpc` (a JSON body with the operation and its parameters, crates/mzlab-desktop/src/rpc.rs), the traces `tic`/`xic`/`spectra` come back as binary.
// A file is known by the number the window gave it: the tag in `File.type` (files picked in the system dialog), the number of a converted .raw
// (`mzlabId`) or, for a File that only exists in the page (dragged in, no tag), the bytes are sent once to the app (`lr/upload`) and get a number.
const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
const CHUNK = 32 << 20;
const tag = id => "application/x-mzlab-file;id=" + id;

const url = (route, p) => BASE + route + (p ? "?" + Object.entries(p).map(([k, v]) => k + "=" + encodeURIComponent(v)).join("&") : "");

// An engine error is a JSON string { error_key, params } (same as the WASM engine); anything else is a fault of the worker.
async function get(route, p) {
  const r = await fetch(url(route, p));
  if (r.status !== 200) throw await r.text();
  return r;
}
async function post(route, p, body) {
  const r = await fetch(url(route, p), { method: "POST", body });
  if (r.status !== 200) throw await r.text();
  return r;
}

const known = new WeakMap();            // Blob -> number (a file sent as bytes is sent once)
async function fileId(blob, name) {
  const m = /id=(\d+)/.exec(blob.type || "");
  if (m) return Number(m[1]);
  if (blob.mzlabId != null) return blob.mzlabId;
  if (known.has(blob)) return known.get(blob);
  const nm = name || blob.name;
  if (!nm) throw JSON.stringify({ error_key: "err.file.raw", params: { detail: "no file" } });
  let id = null;
  for (let off = 0; off === 0 || off < blob.size; off += CHUNK) {
    const j = await (await post("lr/upload", { name: nm, off, total: blob.size }, blob.slice(off, off + CHUNK))).json();
    if (j.id) id = j.id;
  }
  known.set(blob, id);
  return id;
}

// the File the bridge keeps for a file of the app: empty, with its number in `type` (the bridge only passes it back)
const stub = (name, id) => new File([], name, { type: tag(id) });

// The WASM worker does one thing at a time, in the order of the messages (a `reset` sent just before the `idxopen` of the new session cannot overtake it).
// The native engine answers from several threads, so the messages are put in a line here; only the long ones that have a thread pool or a
// progress of their own (index builds, .raw conversions) do not wait for the others.
let line = Promise.resolve();
onmessage = ev => {
  const m = ev.data;
  if (m.op === "recycle") return;
  if (m.op === "lr_index" || m.op === "raw2mzml") { handle(m); return; }
  line = line.then(() => handle(m));
};

async function handle(m) {
  const { id } = m;
  try {
    let buf = null, len = 0, n = 0, info = null, files = null;
    switch (m.op) {
      case "open": info = await (await get("open", { slot: m.slot, id: await fileId(m.blob) })).text(); break;
      case "raw2mzml": {                                                              // .raw -> mzML on disk (native), then read in chunks
        const rid = await fileId(m.blob);
        // the conversion is one long native call: ask now and then how far it has got
        const poll = setInterval(async () => {
          try { const p = await (await get("rawprogress", { id: rid })).json(); if (p.total) postMessage({ id, op: "progress", total: p.total, read: p.read, written: p.written }); } catch (_) { /* optional */ }
        }, 300);
        let out;
        try { out = await (await get("raw2mzml", { id: rid })).json(); } finally { clearInterval(poll); }
        const parts = [];
        for (let off = 0; off < out.size; off += CHUNK) {
          parts.push(new Uint8Array(await (await get("file", { id: out.id, off, len: CHUNK })).arrayBuffer()));
          postMessage({ id, op: "progress", total: 1, read: 1, written: out.size, copied: Math.min(off + CHUNK, out.size), size: out.size });
        }
        postMessage({ id, op: m.op, n: parts.length, parts, tid: out.id }, parts.map(a => a.buffer));
        return;
      }
      case "close": await get("close", { slot: m.slot }); await post("rpc", undefined, JSON.stringify({ op: "close", slot: m.slot })); break;
      case "reset": await get("reset"); await post("rpc", undefined, JSON.stringify({ op: "reset" })); break;
      case "crash": throw new Error("crash hook");                                    // test hook: the supervisor must recover
      case "tic": case "xic": case "spectra": {
        const q = m.op === "tic" ? { slot: m.slot, level: m.level, px: m.px || 0 }
          : m.op === "xic" ? { slot: m.slot, level: m.level, mz: m.mz, tol: m.tol, px: m.px || 0 }
          : { slot: m.slot, level: m.level, i0: m.i0, i1: m.i1, bin: m.bin };
        buf = await (await get(m.op, q)).arrayBuffer();
        len = buf.byteLength / 8;
        n = m.op === "spectra" ? len : len / 2;                                       // a trace is [rt(n), y(n)]
        break;
      }
      default: {                                                                      // every other operation of the page: the native `rpc`
        const p = {};
        for (const [k, v] of Object.entries(m)) if (k !== "id" && k !== "blob") p[k] = v;
        if (m.blob) p.fid = await fileId(m.blob, m.op === "lr_put" ? m.name : undefined);
        const r = await post("rpc", undefined, JSON.stringify(p));
        if (m.op === "scanbin") {
          buf = await r.arrayBuffer(); len = buf.byteLength; n = len;
          postMessage({ id, op: m.op, n, info: null, buf: len ? buf : null, len }, len ? [buf] : []);
          return;
        }
        info = await r.text();
        if (m.op === "lr_boot") files = JSON.parse(info).map(f => [f.name, stub(f.name, f.id)]);
        else if (m.op === "lr_put") files = [[m.name, stub(m.name, JSON.parse(info).id)]];
      }
    }
    postMessage({ id, op: m.op, n, info, buf, len, files }, buf ? [buf] : []);
  } catch (e) {
    const text = typeof e === "string" ? e : String(e && e.message || e);
    postMessage({ id, op: m.op, error: text, fatal: !(typeof e === "string" && e.startsWith("{")) });
  }
}
postMessage({ op: "ready" });
