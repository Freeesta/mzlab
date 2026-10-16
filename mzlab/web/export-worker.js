// @ts-check
// Writes an export file piece by piece into OPFS (mzlab/export/<id>), so the page never holds the whole text (export-pezzi.js is the client).
// Messages: { op: "open", id } (answered with { op: "ack", id }), { op: "put", id, data: string | Uint8Array | Blob } (answered with { op: "ack", id }), { op: "end", id } (answered with { op: "done", id, bytes }),
// { op: "abort", id }, { op: "drop", id }, { op: "clean", olderMs } (removes what an interrupted export left); a failure is answered { op: "fail", id, name, message }.
const handles = /** @type {Map<string, { h: any, at: number }>} */ (new Map());
const enc = new TextEncoder();
const dir = async (/** @type {boolean} */ create) => {
  const root = await navigator.storage.getDirectory();
  return (await root.getDirectoryHandle("mzlab", { create })).getDirectoryHandle("export", { create });
};
const STEP = 8 << 20;             // a Blob is read slice by slice, never whole
onmessage = async ev => {
  const m = ev.data;
  try {
    if (m.op === "open") {
      const fh = await (await dir(true)).getFileHandle(m.id, { create: true });
      const h = await /** @type {any} */ (fh).createSyncAccessHandle(); h.truncate(0);
      handles.set(m.id, { h, at: 0 });
      postMessage({ op: "ack", id: m.id });
    } else if (m.op === "put") {
      const w = handles.get(m.id); if (!w) throw new Error("no file");
      const parts = m.data instanceof Blob ? null : [typeof m.data === "string" ? enc.encode(m.data) : m.data];
      if (parts) for (const b of parts) w.at += w.h.write(b, { at: w.at });
      else for (let o = 0; o < m.data.size; o += STEP) { const b = new Uint8Array(await m.data.slice(o, o + STEP).arrayBuffer()); w.at += w.h.write(b, { at: w.at }); }
      postMessage({ op: "ack", id: m.id });
    } else if (m.op === "end") {
      const w = handles.get(m.id); if (!w) throw new Error("no file");
      w.h.flush(); w.h.close(); handles.delete(m.id);
      postMessage({ op: "done", id: m.id, bytes: w.at });
    } else if (m.op === "abort" || m.op === "drop") {
      const w = handles.get(m.id); if (w) { try { w.h.close(); } catch (_) { /* already closed */ } handles.delete(m.id); }
      try { await (await dir(false)).removeEntry(m.id); } catch (_) { /* gone already */ }
      postMessage({ op: "ack", id: m.id });
    } else if (m.op === "clean") {
      let d; try { d = await dir(false); } catch (_) { return; }
      const old = [];
      for await (const [name, e] of /** @type {any} */ (d).entries()) {
        if (e.kind !== "file" || handles.has(name)) continue;
        if (Date.now() - (await e.getFile()).lastModified > m.olderMs) old.push(name);
      }
      for (const n of old) { try { await d.removeEntry(n); } catch (_) { /* in use */ } }
    }
  } catch (e) {
    const w = handles.get(m.id); if (w) { try { w.h.close(); } catch (_) { /* closed */ } handles.delete(m.id); }
    postMessage({ op: "fail", id: m.id, name: e && /** @type {any} */ (e).name, message: String(e && /** @type {any} */ (e).message || e) });
  }
};
