// Replaces static/rust-worker.js in the desktop app (prepare.py): same messages as the WASM worker, but every answer comes from the
// native engine through the mzlab:// protocol (binary, no JSON; files are memory-mapped from the disk, no 150 MB limit for .raw).
// A file is known by the number the window gave it: the tag in `File.type` (files picked in the system dialog) or its name and size
// (a converted .raw, a file handed over again by the supervisor).
const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
const CHUNK = 32 << 20;

const url = (route, p) => BASE + route + "?" + Object.entries(p || {}).map(([k, v]) => k + "=" + encodeURIComponent(v)).join("&");

// An engine error is a JSON string { error_key, params } (same as the WASM engine): the supervisor leaves that file to Python.
async function get(route, p) {
  const r = await fetch(url(route, p));
  if (r.status !== 200) throw await r.text();
  return r;
}

async function fileId(blob) {
  const m = /id=(\d+)/.exec(blob.type || "");
  if (m) return Number(m[1]);
  if (!blob.name) throw JSON.stringify({ error_key: "err.file.raw", params: { detail: "no file" } });
  return (await (await get("lookup", { name: blob.name, size: blob.size })).json()).id;
}

onmessage = async ev => {
  const m = ev.data;
  if (m.op === "recycle") return;
  const { id } = m;
  try {
    let buf = null, len = 0, n = 0, info = null;
    switch (m.op) {
      case "open": info = await (await get("open", { slot: m.slot, id: await fileId(m.blob) })).text(); break;
      case "raw2mzml": {                                                              // .raw -> mzML on disk (native), then read in chunks
        const rid = await fileId(m.blob);
        // the conversion is one long native call: ask now and then how far it has got (an older app has no such route: nothing is shown)
        const poll = setInterval(async () => {
          try { const p = await (await get("rawprogress", { id: rid })).json(); if (p.total) postMessage({ id, op: "progress", total: p.total, read: p.read, written: p.written, scans: p.scans, scansTotal: p.scansTotal }); } catch (_) { /* optional */ }
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
      case "close": await get("close", { slot: m.slot }); break;
      case "reset": await get("reset"); break;
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
      default: throw new Error("unknown operation " + m.op);
    }
    postMessage({ id, op: m.op, n, info, buf, len }, buf ? [buf] : []);
  } catch (e) {
    const text = typeof e === "string" ? e : String(e && e.message || e);
    postMessage({ id, op: m.op, error: text, fatal: !(typeof e === "string" && e.startsWith("{")) });
  }
};
postMessage({ op: "ready" });
