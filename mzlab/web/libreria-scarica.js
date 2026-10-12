// @ts-check
"use strict";
// Download of the public library packages into the folder (docs/agenti/librerie.md), for the web page (the worker loads this file with importScripts; node loads it in
// tests/test_libreria_scarica.py). The desktop app does the same in Rust (crates/mzlab-desktop/src/scarica.rs): same catalog, same pieces, same checks.
// The catalog `static/librerie/catalogo.json` and its pieces `static/librerie/<id>/NNN.bin` come from the site itself (CSP unchanged) and are only requested on a click.
// Pieces of 4 MiB are checked one by one with their SHA-256, kept in `.mzlab/` while the package is incomplete (resume = the first piece that is missing or wrong),
// assembled into `.mzlab/<id>.new`, checked as a whole and swapped for the old `<id>.mzlib` in one step: the old package stays until the new one is verified.

const SCARICA = (() => {
  const READER = 1;                                  // the highest .mzlib version this page reads (mzlab_lib::format::VERSION)
  const TRIES = 2;
  const err = (key, params) => Object.assign(new Error(key), { key, params: params || {} });
  const hex = u8 => [...new Uint8Array(u8)].map(x => x.toString(16).padStart(2, "0")).join("");
  const sha256 = async u8 => hex(await crypto.subtle.digest("SHA-256", u8));
  const META = ".mzlab";
  const pieceRel = (id, i) => `${META}/${id}.${String(i).padStart(3, "0")}.piece`;

  // the catalog: the same checks as the Rust side (`parse_catalog`): format, plain ids, pieces that belong to their package, sizes that add up
  function parseCatalog(text) {
    let c; try { c = JSON.parse(text); } catch (e) { throw err("lib.dl.err.catalog", { detail: "json" }); }
    if (!c || c.formato !== "librerie/1" || !Array.isArray(c.librerie)) throw err("lib.dl.err.catalog", { detail: String(c && c.formato) });
    const plain = id => typeof id === "string" && /^[A-Za-z0-9_-]+$/.test(id);
    for (const p of c.librerie) {
      if (!plain(p.id) || !Array.isArray(p.pezzi) || !p.sha256 || !p.versione) throw err("lib.dl.err.catalog", { detail: String(p.id) });
      for (const pc of p.pezzi) if (!String(pc.file).startsWith(p.id + "/") || pc.file.includes("..") || pc.file.includes("\\") || !pc.sha256) throw err("lib.dl.err.catalog", { detail: String(pc.file) });
      if (p.pezzi.reduce((s, x) => s + x.dimensione, 0) !== p.dimensione) throw err("lib.dl.err.catalog", { detail: "size of " + p.id });
    }
    for (const e of c.esterne || []) if (!plain(e.id) || !/^https:\/\//.test(e.url || "")) throw err("lib.dl.err.catalog", { detail: String(e.id) });
    c.esterne = c.esterne || [];
    return c;
  }

  // One piece: fetched, checked (size and SHA-256), fetched once more if it is wrong. An aborted request is a cancel, a refused connection is «offline».
  async function fetchPiece(fetchFn, url, pc, signal) {
    for (let t = 1; ; t++) {
      let res;
      try { res = await fetchFn(url, { cache: "no-store", signal }); }
      catch (e) { if (signal && signal.aborted) throw err("lib.dl.err.cancelled"); throw err("lib.dl.err.offline", { detail: String(e && e.message || e) }); }
      if (!res.ok) throw err("lib.dl.err.http", { status: res.status });
      let b; try { b = new Uint8Array(await res.arrayBuffer()); } catch (e) { if (signal && signal.aborted) throw err("lib.dl.err.cancelled"); throw err("lib.dl.err.offline", { detail: String(e && e.message || e) }); }
      if (b.length === pc.dimensione && (await sha256(b)) === pc.sha256) return b;
      if (t >= TRIES) throw err("lib.dl.err.hash", { what: pc.file });
    }
  }

  // Downloads and installs one package of the catalog into `dir` (a directory of libreria-cartella.js). `base` = the address the catalog was read from (folder of `catalogo.json`).
  // `onProgress({id, piece, pieces, bytes, total})`, `beforeReplace()` lets the caller let go of the old file (an open library), `fetchFn` is `fetch` (tests replace it).
  async function downloadPackage({ dir, pkg, base, signal, onProgress, beforeReplace, fetchFn }) {
    fetchFn = fetchFn || ((u, o) => fetch(u, o));
    if ((pkg.lettore_min || 1) > READER) throw err("lib.dl.err.version", { id: pkg.id });
    const have = new Map((await dir.list(META)).map(f => [f.name, f.size])), n = pkg.pezzi.length;
    let done = 0;
    for (let i = 0; i < n; i++) {
      if (signal && signal.aborted) throw err("lib.dl.err.cancelled");
      const pc = pkg.pezzi[i], rel = pieceRel(pkg.id, i);
      let ok = false;
      if (have.get(rel.slice(META.length + 1)) === pc.dimensione) { try { ok = (await sha256(new Uint8Array(await (await dir.file(rel)).slice(0, pc.dimensione).arrayBuffer()))) === pc.sha256; } catch (e) { ok = false; } }
      if (!ok) await dir.write(rel, await fetchPiece(fetchFn, base + pc.file, pc, signal));
      done += pc.dimensione;
      if (onProgress) onProgress({ id: pkg.id, piece: i + 1, pieces: n, bytes: done, total: pkg.dimensione });
    }
    if (signal && signal.aborted) throw err("lib.dl.err.cancelled");
    const newRel = `${META}/${pkg.id}.new`, drop = async () => { await dir.remove(newRel); for (let i = 0; i < n; i++) await dir.remove(pieceRel(pkg.id, i)); };
    const files = []; for (let i = 0; i < n; i++) files.push(await dir.file(pieceRel(pkg.id, i)));
    await dir.write(newRel, new Blob(files));
    const nf = await dir.file(newRel);
    if (nf.size !== pkg.dimensione || (await CARTELLA.sha256Of(nf)) !== pkg.sha256) { await drop(); throw err("lib.dl.err.hash", { what: pkg.id }); }
    const head = await CARTELLA.readHead(nf);
    if (!head) { await drop(); throw err("lib.dl.err.hash", { what: pkg.id }); }
    if (head.minReader > READER) { await dir.remove(newRel); throw err("lib.dl.err.version", { id: pkg.id }); }
    if (beforeReplace) await beforeReplace();
    const final = pkg.id + ".mzlib";
    await dir.replace(newRel, final);
    const f = await dir.file(final), pos = pkg.polarita === 1 ? pkg.spettri : 0, neg = pkg.polarita === -1 ? pkg.spettri : 0;
    await dir.write(`${META}/${final}.pkg.json`, JSON.stringify({ id: pkg.id, versione: pkg.versione, sha256: pkg.sha256, size: f.size, mtime: f.lastModified,
      info: { n: pkg.spettri || head.n, pos, neg, dropped: 0, peaks: pkg.picchi || head.peaks, date: f.lastModified } }));
    for (let i = 0; i < n; i++) await dir.remove(pieceRel(pkg.id, i));
  }
  // what the window shows for a catalog entry given the entries of the folder: «installed» (up to date), «update» (installed, another version) or «new»
  function stateOf(pkg, voci) {
    const v = (voci || []).find(x => x.tipo === "pacchetto" && x.nome === pkg.id + ".mzlib");
    if (!v) return "new";
    return v.stato === "aggiornamento" || (v.versione && v.versione !== pkg.versione) ? "update" : v.stato === "danneggiata" ? "damaged" : "installed";
  }
  return { READER, parseCatalog, fetchPiece, downloadPackage, stateOf, pieceRel };
})();
if (typeof module !== "undefined") module.exports = SCARICA;
