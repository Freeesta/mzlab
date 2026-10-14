// @ts-check
// @ts-strict
"use strict";
// The «Cartella delle librerie» (docs/agenti/librerie.md): ONE folder holds the spectral libraries of the user. It is read by libreria-worker.js (classic script,
// loaded with importScripts; node loads it in tests/test_libreria_cartella.py for the pure parts).
//   desktop app  -> a native folder, read by the native side (memory-mapped) through the mzlab:// protocol (`nativeDir`)
//   Chrome, Edge -> a folder chosen once with showDirectoryPicker (the handle lives in IndexedDB of the page), read and written through the handle (`handleDir`)
//   Safari, Firefox (and the default everywhere) -> the private file system of the browser (OPFS), directory `librerie/` (`handleDir` on it)
// Layout of the folder:
//   <name>.msp|.mgf|.json…      sources of the user (never modified)
//   <name>.mzlib                 packages (downloaded and verified, or exported); the sidecar `.mzlab/<name>.pkg.json` says which version they are
//   .mzlab/<name>.mzlib          the index of a source, written once (read by every browser and by the app)
//   .mzlab/<name>.idx.json       { v, origine: { nome, size, mtime, sha256 }, info, bytes } of that index
// States of an entry: ready (pronta), to index (da_indicizzare), update available (aggiornamento), damaged (danneggiata). A SHA-256 is computed only when the size or
// the date of a file changed since the last time it was seen.

const CARTELLA = (() => {
  const META = ".mzlab", SRC = /\.(msp|mgf|json|jsonl|ndjson)$/i, STATI = ["pronta", "da_indicizzare", "aggiornamento", "danneggiata"];

  // ---------------------------------------------------------------- SHA-256 (streaming: crypto.subtle has no incremental interface)
  const K = new Uint32Array([0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2]);
  class Sha256 {
    constructor() { this.h = new Uint32Array([0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19]); this.buf = new Uint8Array(64); this.fill = 0; this.len = 0; this.w = new Uint32Array(64); }
    block(b, o) {
      const w = this.w, h = this.h;
      for (let i = 0; i < 16; i++) w[i] = (b[o + 4 * i] << 24) | (b[o + 4 * i + 1] << 16) | (b[o + 4 * i + 2] << 8) | b[o + 4 * i + 3];
      for (let i = 16; i < 64; i++) {
        const a = w[i - 15], c = w[i - 2], s0 = ((a >>> 7) | (a << 25)) ^ ((a >>> 18) | (a << 14)) ^ (a >>> 3), s1 = ((c >>> 17) | (c << 15)) ^ ((c >>> 19) | (c << 13)) ^ (c >>> 10);
        w[i] = (w[i - 16] + s0 + w[i - 7] + s1) | 0;
      }
      let a = h[0], b2 = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
      for (let i = 0; i < 64; i++) {
        const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7)), ch = (e & f) ^ (~e & g), t1 = (hh + S1 + ch + K[i] + w[i]) | 0;
        const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10)), mj = (a & b2) ^ (a & c) ^ (b2 & c), t2 = (S0 + mj) | 0;
        hh = g; g = f; f = e; e = (d + t1) | 0; d = c; c = b2; b2 = a; a = (t1 + t2) | 0;
      }
      h[0] = (h[0] + a) | 0; h[1] = (h[1] + b2) | 0; h[2] = (h[2] + c) | 0; h[3] = (h[3] + d) | 0; h[4] = (h[4] + e) | 0; h[5] = (h[5] + f) | 0; h[6] = (h[6] + g) | 0; h[7] = (h[7] + hh) | 0;
    }
    update(u8) {
      this.len += u8.length; let i = 0;
      if (this.fill) { const n = Math.min(64 - this.fill, u8.length); this.buf.set(u8.subarray(0, n), this.fill); this.fill += n; i = n; if (this.fill < 64) return this; this.block(this.buf, 0); this.fill = 0; }
      for (; i + 64 <= u8.length; i += 64) this.block(u8, i);
      if (i < u8.length) { this.buf.set(u8.subarray(i), 0); this.fill = u8.length - i; }
      return this;
    }
    hex() {
      const bits = this.len * 8, pad = new Uint8Array(((this.fill < 56 ? 56 : 120) - this.fill) + 8); pad[0] = 0x80;
      const dv = new DataView(pad.buffer); dv.setUint32(pad.length - 8, Math.floor(bits / 4294967296)); dv.setUint32(pad.length - 4, bits >>> 0);
      this.update(pad);
      return [...this.h].map(x => (x >>> 0).toString(16).padStart(8, "0")).join("");
    }
  }
  async function sha256Of(file, onProgress) {         // a Blob-like file: read in slices of 4 MiB
    if (file.size <= 4 << 20 && typeof crypto !== "undefined" && crypto.subtle) { const d = await crypto.subtle.digest("SHA-256", await file.slice(0, file.size).arrayBuffer()); return [...new Uint8Array(d)].map(x => x.toString(16).padStart(2, "0")).join(""); }
    const h = new Sha256(); const CH = 4 << 20;
    for (let o = 0; o < file.size; o += CH) { h.update(new Uint8Array(await file.slice(o, Math.min(file.size, o + CH)).arrayBuffer())); if (onProgress) onProgress(Math.min(file.size, o + CH), file.size); }
    return h.hex();
  }

  // ---------------------------------------------------------------- states (pure: the callbacks stand for «read the file»)
  // A source: `f` = { size, mtime } of the file, `meta` = the .idx.json (or null), `idxSize` = bytes of the index file (or null if missing), `sha()` = SHA-256 of the source.
  async function statoSorgente(f, meta, idxSize, sha) {
    if (!meta) return { stato: "da_indicizzare" };
    if (idxSize == null || idxSize !== meta.bytes) return { stato: "danneggiata" };           // the index is missing or not what was written
    const o = meta.origine || {};
    if (o.size === f.size && o.mtime === f.mtime) return { stato: "pronta" };
    if (o.size === f.size && o.sha256 && (await sha()) === o.sha256) return { stato: "pronta", mtime: f.mtime };   // touched, not changed
    return { stato: "da_indicizzare" };
  }
  // A package (`.mzlib` at the top of the folder): `side` = its sidecar { id, versione, sha256, size, mtime } (or null for a file of the user), `cat` = the catalog entry
  // of that id (if the catalog is known), `headOk` = the file starts with the magic of the format, `sha()` = its SHA-256.
  async function statoPacchetto(f, side, cat, headOk, sha) {
    if (!side) return { stato: headOk ? "pronta" : "danneggiata" };
    if (f.size !== side.size) return { stato: "danneggiata" };
    let mtime;
    if (f.mtime !== side.mtime) { if ((await sha()) !== side.sha256) return { stato: "danneggiata" }; mtime = f.mtime; }
    const newer = cat && (cat.versione !== side.versione || cat.sha256 !== side.sha256);
    return { stato: newer ? "aggiornamento" : "pronta", mtime, nuova: newer ? cat.versione : undefined };
  }
  const idOf = name => "f:" + name;
  const nameOfId = id => id.slice(2);
  const isFolderId = id => typeof id === "string" && id.startsWith("f:");
  const stem = name => name.replace(/\.(msp|mgf|json|jsonl|ndjson|txt|mzlib)$/i, "");
  // only plain names inside the folder: no «..», no absolute path, no backslash; at most one level (`.mzlab/x`)
  function safeRel(rel) {
    const parts = String(rel).split("/");
    if (!rel || parts.length > 2 || parts.some(p => !p || p === "." || p === ".." || /[\\\0:]/.test(p))) throw new Error("bad path");
    if (parts.length === 2 && parts[0] !== META) throw new Error("bad path");
    return rel;
  }

  // ---------------------------------------------------------------- directories (same interface for handles and for the native folder)
  const splitRel = rel => { const i = rel.lastIndexOf("/"); return [i < 0 ? "" : rel.slice(0, i), rel.slice(i + 1)]; };
  function handleDir(root, kind) {                      // a FileSystemDirectoryHandle (OPFS `librerie/` or the folder chosen with showDirectoryPicker)
    const walk = async (sub, create) => { let d = root; for (const p of sub.split("/").filter(Boolean)) d = await d.getDirectoryHandle(p, { create }); return d; };
    const bytes = async data => (typeof data === "string" ? new TextEncoder().encode(data) : data instanceof Blob ? new Uint8Array(await data.arrayBuffer()) : data);
    const self = {
      kind, name: root.name,
      async list(sub = "") {
        let d; try { d = await walk(sub, false); } catch (e) { return []; }
        const out = []; for await (const [name, h] of d.entries()) if (h.kind === "file") { const f = await h.getFile(); out.push({ name, size: f.size, mtime: f.lastModified }); }
        return out;
      },
      async file(rel) { safeRel(rel); const [sub, name] = splitRel(rel); return (await (await walk(sub, false)).getFileHandle(name)).getFile(); },
      async write(rel, data) {
        safeRel(rel); const [sub, name] = splitRel(rel), fh = await (await walk(sub, true)).getFileHandle(name, { create: true });
        if (fh.createWritable) { const w = await fh.createWritable(); await w.write(data); await w.close(); return; }
        const a = await fh.createSyncAccessHandle(); a.truncate(0);
        if (data instanceof Blob) { const CH = 8 << 20; for (let o = 0; o < data.size; o += CH) a.write(new Uint8Array(await data.slice(o, o + CH).arrayBuffer()), { at: o }); } else a.write(await bytes(data), { at: 0 });
        a.flush(); a.close();
      },
      async remove(rel) { safeRel(rel); const [sub, name] = splitRel(rel); try { await (await walk(sub, false)).removeEntry(name); } catch (e) { /* not there */ } },
      async replace(tmp, rel) {                         // tmp -> rel: the old content stays until the new file is complete
        safeRel(tmp); safeRel(rel); const [sub, name] = splitRel(rel);
        try { const fh = await (await walk(splitRel(tmp)[0], false)).getFileHandle(splitRel(tmp)[1]); if (fh.move) { await fh.move(await walk(sub, true), name); return; } } catch (e) { /* copy instead */ }
        await self.write(rel, await self.file(tmp)); await self.remove(tmp);
      },
      reader: file => { const fr = new FileReaderSync(); return (off, len) => new Uint8Array(fr.readAsArrayBuffer(file.slice(off, off + len))); },
    };
    return self;
  }
  function nativeDir(base) {                            // the folder of the desktop app, through mzlab://localhost/lib/<route> (crates/mzlab-desktop/src/librerie.rs)
    const url = (route, p) => base + "lib/" + route + "?" + Object.entries(p || {}).map(([k, v]) => k + "=" + encodeURIComponent(v)).join("&");
    const get = async (route, p) => { const r = await fetch(url(route, p)); if (r.status !== 200) throw new Error(await r.text()); return r; };
    const CH = 8 << 20;
    const syncRead = (rel, off, len) => { const x = new XMLHttpRequest(); x.open("GET", url("read", { name: rel, off, len }), false); x.responseType = "arraybuffer"; x.send(); if (x.status !== 200) throw new Error("read"); return new Uint8Array(x.response); };
    const asyncRead = async (rel, off, len) => new Uint8Array(await (await get("read", { name: rel, off, len })).arrayBuffer());
    const wrap = (rel, st) => ({
      name: splitRel(rel)[1], size: st.size, lastModified: st.mtime, rel,
      slice(a, b) { const s = Math.max(0, a), e = Math.min(st.size, b); return { size: e - s, arrayBuffer: async () => (await asyncRead(rel, s, e - s)).buffer, text: async () => new TextDecoder().decode(await asyncRead(rel, s, e - s)) }; },
      stream() { let off = 0; return { getReader: () => ({ read: async () => { if (off >= st.size) return { done: true }; const v = await asyncRead(rel, off, Math.min(CH, st.size - off)); off += v.length; return { done: false, value: v }; } }) }; },
      async text() { return new TextDecoder().decode(await asyncRead(rel, 0, st.size)); },
      readSync: (off, len) => syncRead(rel, off, len),
    });
    return {
      kind: "native", name: "",
      async list(sub = "") { return (await (await get("list", { sub })).json()).files; },
      async file(rel) { safeRel(rel); return wrap(rel, await (await get("stat", { name: rel })).json()); },
      async write(rel, data) {
        safeRel(rel); const u8 = typeof data === "string" ? new TextEncoder().encode(data) : null, total = u8 ? u8.length : data.size;
        for (let off = 0; off === 0 || off < total; off += CH) {
          const part = u8 ? u8.subarray(off, off + CH) : new Uint8Array(await data.slice(off, off + CH).arrayBuffer());
          const r = await fetch(url("put", { name: rel, off, total }), { method: "POST", body: part }); if (r.status !== 200) throw new Error(await r.text());
          if (total === 0) break;
        }
      },
      async remove(rel) { safeRel(rel); await get("remove", { name: rel }); },
      async replace(tmp, rel) { safeRel(tmp); safeRel(rel); await get("rename", { from: tmp, to: rel }); },
      reader: file => (off, len) => file.readSync(off, len),
    };
  }

  // ---------------------------------------------------------------- the inventory
  const readJson = async (dir, rel) => { try { return JSON.parse(await (await dir.file(rel)).text()); } catch (e) { return null; } };
  const HEAD = [0x4d, 0x5a, 0x4c, 0x42];
  async function readHead(file) {                      // the 36-byte header of a .mzlib (docs/agenti/librerie.md): null if it is not one
    try { const b = new Uint8Array(await file.slice(0, 36).arrayBuffer()); if (b.length < 36 || !HEAD.every((x, i) => b[i] === x)) return null; const dv = new DataView(b.buffer); return { n: dv.getUint32(8, true), peaks: Number(dv.getBigUint64(16, true)), minReader: dv.getUint16(6, true) }; } catch (e) { return null; }
  }
  // The state of every entry. `cat` = the catalog (or null). Returns { voci: [{ nome, tipo, id, size, mtime, stato, info, nuova }] } sorted by name.
  async function scan(dir, cat) {
    const top = await dir.list(""), sub = await dir.list(META), subSize = new Map(sub.map(f => [f.name, f.size]));
    const catBy = new Map(((cat && cat.librerie) || []).map(e => [e.id, e])), voci = [], seen = new Set();
    for (const f of top) {
      if (f.name.startsWith(".")) continue;
      if (SRC.test(f.name)) {
        const meta = subSize.has(f.name + ".idx.json") ? await readJson(dir, META + "/" + f.name + ".idx.json") : null;
        const st = await statoSorgente(f, meta, subSize.has(f.name + ".mzlib") ? subSize.get(f.name + ".mzlib") : null, async () => sha256Of(await dir.file(f.name)));
        if (st.mtime != null && meta) { meta.origine.mtime = st.mtime; await dir.write(META + "/" + f.name + ".idx.json", JSON.stringify(meta)); }
        seen.add(f.name);
        voci.push({ nome: f.name, tipo: "sorgente", id: idOf(f.name), size: f.size, mtime: f.mtime, stato: st.stato, info: st.stato === "pronta" || st.stato === "danneggiata" ? meta && meta.info : null });
      } else if (/\.mzlib$/i.test(f.name)) {
        const side = await readJson(dir, META + "/" + f.name + ".pkg.json"), file = await dir.file(f.name), head = await readHead(file);
        const st = await statoPacchetto(f, side, side && catBy.get(side.id), !!head, () => sha256Of(file));
        if (st.mtime != null && side) { side.mtime = st.mtime; await dir.write(META + "/" + f.name + ".pkg.json", JSON.stringify(side)); }
        voci.push({ nome: f.name, tipo: "pacchetto", id: idOf(f.name), size: f.size, mtime: f.mtime, stato: st.stato, versione: side && side.versione, nuova: st.nuova, info: st.stato === "pronta" || st.stato === "aggiornamento" ? (side && side.info) || (head && { n: head.n, pos: 0, neg: 0, dropped: 0, peaks: head.peaks }) : null });
        seen.add(f.name);
      }
    }
    for (const f of sub) {                              // indexes of files that are not in the folder (imported with the button or by drop): nothing to compare, ready while the index is whole
      const m = /^(.*)\.idx\.json$/.exec(f.name); if (!m || seen.has(m[1])) continue;
      const meta = await readJson(dir, META + "/" + f.name), ok = meta && subSize.get(m[1] + ".mzlib") === meta.bytes;
      voci.push({ nome: m[1], tipo: "importata", id: idOf(m[1]), size: meta ? meta.origine.size : 0, mtime: meta ? meta.origine.mtime : 0, stato: ok ? "pronta" : "danneggiata", info: meta && meta.info });
    }
    return { voci: voci.sort((a, b) => a.nome.localeCompare(b.nome)) };
  }
  // the file the engine reads for an entry of the folder: the package itself, or the index of a source
  const blobPath = voce => (voce.tipo === "pacchetto" ? voce.nome : META + "/" + voce.nome + ".mzlib");
  const metaOf = (name, size, mtime, sha, info, bytes) => ({ v: 1, origine: { nome: name, size, mtime, sha256: sha }, info, bytes, date: Date.now() });

  return { Sha256, sha256Of, readHead, statoSorgente, statoPacchetto, scan, handleDir, nativeDir, idOf, nameOfId, isFolderId, stem, safeRel, blobPath, metaOf, META, SRC, STATI };
})();
if (typeof module !== "undefined") module.exports = CARTELLA;
