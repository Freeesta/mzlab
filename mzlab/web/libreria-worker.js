"use strict";
// Spectral libraries (MSP, MGF, JSON, MassBank records) read in a Web Worker, kept in the browser's private file system (OPFS) and searched with a MS2 spectrum.
// No Pyodide. Where WebAssembly works the work is done by static/wasm-lib/ (crates/mzlab-lib-wasm: the .mzlib format of docs/agenti/librerie.md, Flash indexes, scores);
// the plain JavaScript below (parser, scores, search over an index: also run by node in tests/test_libreria.py) is the fallback for browsers that cannot load it,
// and still reads the libraries stored in the old MZL1 archive format until they are converted into .mzlib (once, at the first listing).
//
// Archive of a library (OPFS, or IndexedDB where OPFS is not available): per library `id`
//   <id>.idx        header (magic, n, total peaks) + precursor m/z SORTED (Float64) + their position in the file (Uint32) + polarity (Int8) + peak offsets (Uint32, in file order)
//   <id>.peaks      the packed peaks, in file order: Float32 pairs (m/z, intensity), at most 100 per spectrum, intensity normalised to 1
//   <id>.meta.json  name, type (adduct), CE, instrument, formula, InChIKey of every spectrum (file order)
//   <id>.info.json  the summary shown in the list (name, spectra, polarities, discarded, date)
// A spectrum is: { prec, pol (+1 / -1 / 0), mz: Float32Array, it: Float32Array, meta: [name, type, ce, inst, formula, inchikey] }.

const TOP_PEAKS = 100;
const MAGIC = 0x4d5a4c31;                       // "MZL1"
const NUM = /[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g;

// ---------------------------------------------------------------- parsing
const keyOf = k => k.toLowerCase().replace(/[\s_\-]+/g, "");
function polFrom(mode, type, charge) {
  const m = String(mode || "").trim();
  if (/^p/i.test(m)) return 1;
  if (/^n/i.test(m)) return -1;
  const t = String(type || "").match(/\][\d]*([+-])\s*$/); if (t) return t[1] === "+" ? 1 : -1;
  const c = String(charge || "").match(/([+-])/); if (c) return c[1] === "+" ? 1 : -1;
  return 0;
}
function topPeaks(mz, it) {                     // the TOP_PEAKS most intense, by m/z, intensity divided by the highest
  let idx = Array.from(mz.keys());
  if (idx.length > TOP_PEAKS) idx = idx.sort((a, b) => it[b] - it[a]).slice(0, TOP_PEAKS);
  idx.sort((a, b) => mz[a] - mz[b]);
  const mx = Math.max(...idx.map(i => it[i]));
  return { mz: Float32Array.from(idx, i => mz[i]), it: Float32Array.from(idx, i => it[i] / mx) };
}
class LibParser {
  // format "msp" | "mgf"; onSpectrum(spectrum) is called for every good one; push(text) as the file is read, end() at the end.
  constructor(format, onSpectrum) {
    this.fmt = format; this.cb = onSpectrum; this.rest = ""; this.cur = null; this.inPeaks = false;
    this.stats = { read: 0, dropped: 0, noPrec: 0, noPeaks: 0, broken: 0 };
  }
  push(text) { const lines = (this.rest + text).split(/\r?\n/); this.rest = lines.pop(); for (const l of lines) this.line(l); }
  end() { if (this.rest) { this.line(this.rest); this.rest = ""; } if (this.cur) { if (this.fmt === "mgf") this.drop("broken"); else this.emit(); } }
  fresh() { this.cur = { f: {}, mz: [], it: [] }; this.inPeaks = false; }
  drop(why) { this.stats.dropped++; if (why) this.stats[why]++; this.cur = null; this.inPeaks = false; }
  field(k, v) { const c = this.cur; if (!(k in c.f)) c.f[k] = v; }
  peak(t) {
    for (const seg of t.split(";")) { const n = seg.match(NUM); if (n && n.length >= 2) { const m = +n[0], i = +n[1]; if (Number.isFinite(m) && Number.isFinite(i) && i > 0 && m > 0) { this.cur.mz.push(m); this.cur.it.push(i); } } }
  }
  line(l) { return this.fmt === "mgf" ? this.mgf(l) : this.msp(l); }
  msp(l) {
    const t = l.trim();
    if (!t) { if (this.cur) this.emit(); return; }
    const m = /^([A-Za-z][A-Za-z0-9 _\-]*?)\s*:\s*(.*)$/.exec(t);
    if (m && (!this.inPeaks || keyOf(m[1]) === "name")) {                    // a «Name:» line always starts a new record, even without the blank line
      const k = keyOf(m[1]);
      if (this.cur && k === "name" && (this.inPeaks || "name" in this.cur.f)) this.emit();
      if (!this.cur) this.fresh();
      if (k === "numpeaks") { this.inPeaks = true; return; }
      this.field(k, m[2].trim()); return;
    }
    if (!this.cur) { if (!/^[-+.\d]/.test(t)) return; this.fresh(); }          // numbers without any header: a record with no precursor (it will be counted)
    this.inPeaks = true; this.peak(t);
  }
  mgf(l) {
    const t = l.trim();
    if (!t) return;
    if (/^BEGIN IONS/i.test(t)) { if (this.cur) this.drop("broken"); this.fresh(); return; }
    if (/^END IONS/i.test(t)) { if (this.cur) this.emit(); return; }
    if (!this.cur) return;
    const m = /^([A-Za-z_][A-Za-z0-9_ \-]*)=(.*)$/.exec(t);
    if (m && !/^[-+.\d]/.test(t)) {
      const k = keyOf(m[1]); let v = m[2].trim();
      if (k === "pepmass") { const n = v.match(NUM); if (n) v = n[0]; this.field("precursormz", v); } else this.field(k, v);
      return;
    }
    this.inPeaks = true; this.peak(t);
  }
  emit() {
    const c = this.cur, f = c.f; this.cur = null; this.inPeaks = false;
    const prec = parseFloat(String(f.precursormz ?? f.precursor ?? "").replace(",", "."));
    if (!(prec > 0)) { this.stats.dropped++; this.stats.noPrec++; return; }
    if (!c.mz.length) { this.stats.dropped++; this.stats.noPeaks++; return; }
    const p = topPeaks(c.mz, c.it), type = f.precursortype ?? f.adduct ?? "";
    this.stats.read++;
    this.cb({ prec, pol: polFrom(f.ionmode ?? f.polarity, type, f.charge), mz: p.mz, it: p.it,
      meta: [f.name ?? f.title ?? "", type, f.collisionenergy ?? f.ce ?? "", f.instrumenttype ?? f.instrument ?? "", f.formula ?? "", f.inchikey ?? "", f.db ?? f.accession ?? f.accessionnumber ?? "", f.authors ?? f.author ?? f.datacollector ?? "", f.license ?? f.licence ?? ""] });
  }
}
function formatOf(name, head) {                 // by extension, otherwise by the first lines
  const n = String(name || "").toLowerCase();
  if (/\.msp$/.test(n)) return "msp";
  if (/\.mgf$/.test(n)) return "mgf";
  if (/\.(lib|mzvault|sqlite|db|nist)$/.test(n)) return "unsupported";
  const h = String(head || "");
  if (/BEGIN IONS/i.test(h)) return "mgf";
  if (/^\s*name\s*:/im.test(h)) return "msp";
  return "unknown";
}

// ---------------------------------------------------------------- scores
const lowerBound = (a, x) => { let lo = 0, hi = a.length; while (lo < hi) { const m = (lo + hi) >> 1; if (a[m] < x) lo = m + 1; else hi = m; } return lo; };
function matchPeaks(aMz, aIt, bMz, bIt, tol) {  // one-to-one pairs within tol (Da), the most intense pairs first
  const c = [];
  for (let i = 0; i < aMz.length; i++) for (let j = lowerBound(bMz, aMz[i] - tol); j < bMz.length && bMz[j] <= aMz[i] + tol; j++) c.push([i, j, aIt[i] * bIt[j]]);
  c.sort((x, y) => y[2] - x[2]);
  const ua = new Uint8Array(aMz.length), ub = new Uint8Array(bMz.length), out = [];
  for (const [i, j] of c) if (!ua[i] && !ub[j]) { ua[i] = ub[j] = 1; out.push([i, j]); }
  return out;
}
function cosine(a, b, pairs) {                  // intensities as square roots
  let na = 0, nb = 0, dot = 0;
  for (const v of a.it) na += v; for (const v of b.it) nb += v;                // sqrt(I)^2 = I
  for (const [i, j] of pairs) dot += Math.sqrt(a.it[i] * b.it[j]);
  return na > 0 && nb > 0 ? dot / Math.sqrt(na * nb) : 0;
}
function entropyWeighted(it) {                  // Li et al., Nat. Methods 2021: sum 1; if the entropy is below 3 the intensities are raised to 0.25 + 0.25 S
  const s = it.reduce((x, y) => x + y, 0); let p = Array.from(it, v => v / s);
  const S = -p.reduce((x, v) => x + (v > 0 ? v * Math.log(v) : 0), 0);
  if (S < 3) { const w = 0.25 + 0.25 * S; p = p.map(v => Math.pow(v, w)); const t = p.reduce((x, y) => x + y, 0); p = p.map(v => v / t); }
  return p;
}
function entropySimilarity(a, b, pairs) {       // 1 - (2 S_AB - S_A - S_B) / ln 4
  const pa = entropyWeighted(a.it), pb = entropyWeighted(b.it), H = p => -p.reduce((x, v) => x + (v > 0 ? v * Math.log(v) : 0), 0);
  const m = [], ia = new Set(), ib = new Set();
  for (const [i, j] of pairs) { m.push((pa[i] + pb[j]) / 2); ia.add(i); ib.add(j); }
  pa.forEach((v, i) => { if (!ia.has(i)) m.push(v / 2); }); pb.forEach((v, j) => { if (!ib.has(j)) m.push(v / 2); });
  const s = 1 - (2 * H(m) - H(pa) - H(pb)) / Math.log(4);
  return Math.max(0, Math.min(1, s));
}
function score(q, lib, tolFrag) {
  const pairs = matchPeaks(q.mz, q.it, lib.mz, lib.it, tolFrag);
  let dot = 0, qp = 0, qa = 0, la = 0;                         // reverse = the paired peaks of the query against ALL the library peaks; explained = the share of the query intensity that is paired
  for (const [i, j] of pairs) { dot += Math.sqrt(q.it[i] * lib.it[j]); qp += q.it[i]; }
  for (const v of q.it) qa += v; for (const v of lib.it) la += v;
  return { cos: cosine(q, lib, pairs), ent: entropySimilarity(q, lib, pairs), shared: pairs.length, pairs, reverse: qp > 0 && la > 0 ? dot / Math.sqrt(qp * la) : 0, explained: qa > 0 ? qp / qa : 0 };
}

// ---------------------------------------------------------------- search over an index
// idx = { n, prec (sorted Float64Array), ord (Uint32Array: position in the file), pol (Int8Array, file order) }; getSpec(position) -> { mz, it }
// opts = { ppm (precursor tolerance in ppm) or da, frag (Da), pol (+1 / -1 / 0 = any) }
function searchIndex(idx, q, opts, getSpec, meta) {
  const tol = opts.ppm != null ? q.prec * opts.ppm * 1e-6 : opts.da, lo = lowerBound(idx.prec, q.prec - tol), out = [];
  for (let k = lo; k < idx.n && idx.prec[k] <= q.prec + tol; k++) {
    const pos = idx.ord[k], lp = idx.pol[pos];
    if (opts.pol && lp && lp !== opts.pol) continue;                           // same polarity (a library entry without polarity is kept)
    const s = getSpec(pos), sc = score(q, s, opts.frag), m = meta ? meta[pos] : null;
    out.push({ pos, name: m ? m[0] : "", type: m ? m[1] : "", ce: m ? m[2] : "", inst: m ? m[3] : "", formula: m ? m[4] : "", ik: m ? m[5] : "", accession: m ? m[6] || "" : "", authors: m ? m[7] || "" : "", license: m ? m[8] || "" : "",
      cos: sc.cos, ent: sc.ent, shared: sc.shared, reverse: sc.reverse, explained: sc.explained, polarity: lp, dprec: opts.ppm != null ? (idx.prec[k] - q.prec) / q.prec * 1e6 : idx.prec[k] - q.prec, prec: idx.prec[k], pairs: sc.pairs, mz: Array.from(s.mz), it: Array.from(s.it) });
  }
  return out;
}
// ---------------------------------------------------------------- analogues: modified cosine (match at shift 0 and at shift delta = precursor difference)
const AN_TOP = 32;                               // peaks used for the modified cosine: the most intense, sorted by m/z
function top32(s, prec) {                        // the precursor region (within 1.5 of it) and the noise (< 1% of the highest) are left out, then the 32 most intense
  const mx = Math.max(0, ...s.it);
  let idx = Array.from(s.mz.keys()).filter(i => s.mz[i] < prec - 1.5 && s.it[i] >= 0.01 * mx && s.it[i] > 0);
  if (idx.length > AN_TOP) idx = idx.sort((a, b) => s.it[b] - s.it[a]).slice(0, AN_TOP);
  idx.sort((a, b) => s.mz[a] - s.mz[b]);
  return { mz: Float64Array.from(idx, i => s.mz[i]), it: Float64Array.from(idx, i => s.it[i]) };
}
// prefilter at linear cost: fragments in common (within tol) + neutral losses in common (precursor - m/z), two pointers on sorted arrays
function sharedCount(aMz, bMz, tol) {
  let i = 0, j = 0, n = 0;
  while (i < aMz.length && j < bMz.length) { const d = aMz[i] - bMz[j]; if (Math.abs(d) <= tol) { n++; i++; j++; } else if (d < 0) i++; else j++; }
  return n;
}
function lossesOf(mz, prec) { const l = Float64Array.from(mz, m => prec - m); l.reverse(); return l; }       // ascending: mz is ascending
function coincidences(q, lib, tol) { return sharedCount(q.mz, lib.mz, tol) + sharedCount(lossesOf(q.mz, q.prec), lossesOf(lib.mz, lib.prec), tol); }
// modified cosine: candidate pairs at shift 0 and shift delta, one-to-one greedy by decreasing weight sqrt(Iq*Il), normalised as the cosine of the square roots
function modCosine(q, lib, tol) {
  const a = top32(q, q.prec), b = top32(lib, lib.prec), delta = q.prec - lib.prec, c = [];
  for (let i = 0; i < a.mz.length; i++) for (const sh of (Math.abs(delta) < 1e-6 ? [0] : [0, delta])) {
    const t = a.mz[i] - sh;
    for (let j = lowerBound(b.mz, t - tol); j < b.mz.length && b.mz[j] <= t + tol; j++) c.push([i, j, Math.sqrt(a.it[i] * b.it[j]), sh !== 0]);
  }
  c.sort((x, y) => y[2] - x[2]);
  const ua = new Uint8Array(a.mz.length), ub = new Uint8Array(b.mz.length), pairs = []; let dot = 0, shifted = 0;
  for (const [i, j, w, sh] of c) if (!ua[i] && !ub[j]) { ua[i] = ub[j] = 1; dot += w; pairs.push([i, j, sh]); if (sh) shifted++; }
  let na = 0, nb = 0; for (const v of a.it) na += v; for (const v of b.it) nb += v;
  return { cos: na > 0 && nb > 0 ? dot / Math.sqrt(na * nb) : 0, matched: pairs.length, shifted, delta, pairs, a, b };
}
// analogues of the query q = {prec, pol, mz, it} in an index: candidates with |delta precursor| <= maxDelta and the same polarity, the 300 best after the prefilter
// (>= 3 coincidences), then the exact modified cosine (>= 4 matched peaks). getSpec(position) -> {mz, it}
function analogIndex(idx, q, opts, getSpec, meta) {
  const maxD = opts.maxDelta ?? 200, frag = opts.frag ?? 0.01, lo = lowerBound(idx.prec, q.prec - maxD), cand = [];
  const qs = { mz: q.mz, prec: q.prec };
  for (let k = lo; k < idx.n && idx.prec[k] <= q.prec + maxD; k++) {
    const pos = idx.ord[k], lp = idx.pol[pos];
    if (opts.pol && lp && lp !== opts.pol) continue;
    if (Math.abs(idx.prec[k] - q.prec) < (opts.minDelta ?? 0.5)) continue;                // the same mass is an identity, not an analogue
    const s = getSpec(pos); if (!s) continue;
    const n = coincidences(qs, { mz: s.mz, prec: idx.prec[k] }, frag);
    if (n >= 3) cand.push({ pos, k, n, s });
  }
  cand.sort((x, y) => y.n - x.n); cand.length = Math.min(cand.length, 300);
  const out = [];
  for (const c of cand) {
    const lib = { mz: c.s.mz, it: c.s.it, prec: idx.prec[c.k] }, r = modCosine({ mz: q.mz, it: q.it, prec: q.prec }, lib, frag);
    if (r.matched < 4) continue;
    const m = meta ? meta[c.pos] : null;
    out.push({ pos: c.pos, name: m ? m[0] : "", type: m ? m[1] : "", ce: m ? m[2] : "", inst: m ? m[3] : "", formula: m ? m[4] : "", ik: m ? m[5] : "", accession: m ? m[6] || "" : "", authors: m ? m[7] || "" : "", license: m ? m[8] || "" : "", mcos: r.cos, matched: r.matched, shifted: r.shifted,
      delta: r.delta, prec: idx.prec[c.k], coinc: c.n, mz: Array.from(c.s.mz), it: Array.from(c.s.it), pairs: r.pairs.map(p => [p[0], p[1], p[2]]), qa: { mz: Array.from(r.a.mz), it: Array.from(r.a.it) }, la: { mz: Array.from(r.b.mz), it: Array.from(r.b.it) } });
  }
  out.sort((x, y) => y.mcos - x.mcos);
  return out;
}
function queryOf(peaks, prec, pol) {            // peaks: [[mz, intensity], ...] -> the query, with the same normalisation as the library
  const mz = peaks.map(p => p[0]), it = peaks.map(p => p[1]).map(Number);
  if (!mz.length || !(Math.max(...it) > 0)) return null;
  const t = topPeaks(mz.filter((_, i) => it[i] > 0), it.filter(v => v > 0));
  return { prec, pol, mz: t.mz, it: t.it };
}

if (typeof module !== "undefined") module.exports = { modCosine, analogIndex, coincidences, LibParser, formatOf, topPeaks, matchPeaks, cosine, entropySimilarity, score, searchIndex, queryOf, lowerBound, buildIndex: (...a) => buildIndex(...a), packIndex: (...a) => packIndex(...a), unpackIndex: (...a) => unpackIndex(...a), polFrom };

// ---------------------------------------------------------------- index (built after reading a library)
function buildIndex(prec, pol, offsets) {       // prec/pol/offsets in file order -> the sorted arrays
  const n = prec.length, ord = Uint32Array.from({ length: n }, (_, i) => i).sort((a, b) => prec[a] - prec[b] || a - b);
  return { n, prec: Float64Array.from(ord, i => prec[i]), ord, pol: Int8Array.from(pol), off: Uint32Array.from(offsets) };
}
const pad8 = n => (n + 7) & ~7;
function packIndex(idx, totalPeaks) {
  const n = idx.n, o1 = 16, o2 = o1 + n * 8, o3 = o2 + pad8(n * 4), o4 = o3 + pad8(n), size = o4 + (n + 1) * 4, buf = new ArrayBuffer(pad8(size));
  new Uint32Array(buf, 0, 4).set([MAGIC, n, totalPeaks, 0]);
  new Float64Array(buf, o1, n).set(idx.prec); new Uint32Array(buf, o2, n).set(idx.ord); new Int8Array(buf, o3, n).set(idx.pol); new Uint32Array(buf, o4, n + 1).set(idx.off);
  return buf;
}
function unpackIndex(buf) {
  const h = new Uint32Array(buf, 0, 4); if (h[0] !== MAGIC) throw Object.assign(new Error("invalid archive"), { key: "lib.err.archive" });
  const n = h[1], o1 = 16, o2 = o1 + n * 8, o3 = o2 + pad8(n * 4), o4 = o3 + pad8(n);
  return { n, totalPeaks: h[2], prec: new Float64Array(buf, o1, n), ord: new Uint32Array(buf, o2, n), pol: new Int8Array(buf, o3, n), off: new Uint32Array(buf, o4, n + 1) };
}

// ---------------------------------------------------------------- storage: OPFS, or IndexedDB where it is not there
if (typeof self !== "undefined" && typeof self.postMessage === "function" && typeof importScripts === "function") {
  importScripts("libreria-cartella.js", "libreria-scarica.js");           // the folder of the libraries (CARTELLA) and the download of the packages (SCARICA)
  const hasOpfs = () => !!(self.navigator && navigator.storage && navigator.storage.getDirectory);
  let opfsOk = null;
  const root = () => navigator.storage.getDirectory();
  async function probeOpfs() {                  // Safari / old browsers: no synchronous handles in a worker -> IndexedDB
    if (opfsOk !== null) return opfsOk;
    try { const h = await (await root()).getFileHandle("probe.tmp", { create: true }); const a = await h.createSyncAccessHandle(); a.close(); opfsOk = true; } catch (e) { opfsOk = false; }
    return opfsOk;
  }
  const idb = () => new Promise((ok, no) => { const r = indexedDB.open("mzlab-libs", 1); /* kept as is: renaming would lose users' libraries */ r.onupgradeneeded = () => r.result.createObjectStore("lib"); r.onsuccess = () => ok(r.result); r.onerror = () => no(r.error); });
  const idbDo = async (mode, fn) => { const db = await idb(); return new Promise((ok, no) => { const t = db.transaction("lib", mode), s = t.objectStore("lib"), r = fn(s); t.oncomplete = () => { db.close(); ok(r && r.result); }; t.onerror = () => no(t.error); }); };

  class Sink {                                  // the packed peaks are written while the file is read (little memory)
    constructor(id) { this.id = id; this.chunks = []; this.buf = []; this.size = 0; this.h = null; this.at = 0; }
    async open() { if (await probeOpfs()) { const f = await (await root()).getFileHandle(this.id + ".peaks", { create: true }); this.h = await f.createSyncAccessHandle(); this.h.truncate(0); } }
    flush(force) {
      if (!this.buf.length) return;
      if (!force && this.size < 8e6) return;
      const all = new Float32Array(this.size); let o = 0; for (const b of this.buf) { all.set(b, o); o += b.length; }
      if (this.h) { this.h.write(new Uint8Array(all.buffer), { at: this.at }); this.at += all.byteLength; } else this.chunks.push(all);
      this.buf = []; this.size = 0;
    }
    add(mz, it) { const a = new Float32Array(mz.length * 2); for (let i = 0; i < mz.length; i++) { a[2 * i] = mz[i]; a[2 * i + 1] = it[i]; } this.buf.push(a); this.size += a.length; this.flush(false); }
    async close() { this.flush(true); if (this.h) { this.h.flush(); this.h.close(); this.h = null; return null; } const all = new Float32Array(this.chunks.reduce((s, c) => s + c.length, 0)); let o = 0; for (const c of this.chunks) { all.set(c, o); o += c.length; } return all.buffer; }
  }
  const writeSmall = async (name, data) => { const f = await (await root()).getFileHandle(name, { create: true }); const a = await f.createSyncAccessHandle(); a.truncate(0); const u = typeof data === "string" ? new TextEncoder().encode(data) : new Uint8Array(data); a.write(u, { at: 0 }); a.flush(); a.close(); };
  const readFile = async name => (await (await root()).getFileHandle(name)).getFile();

  async function saveLibrary(id, info, idxBuf, meta, peaksBuf) {
    if (opfsOk) { await writeSmall(id + ".idx", idxBuf); await writeSmall(id + ".meta.json", JSON.stringify(meta)); await writeSmall(id + ".info.json", JSON.stringify(info)); }
    else await idbDo("readwrite", s => s.put({ info, idx: idxBuf, meta, peaks: peaksBuf }, id));
  }
  async function listLibraries() {
    const out = [];
    if (await probeOpfs()) { const d = await root(); for await (const [name] of d.entries()) if (name.endsWith(".info.json")) { try { out.push(JSON.parse(await (await (await d.getFileHandle(name)).getFile()).text())); } catch (e) { /* damaged: ignored */ } } }
    else { const db = await idb(); await new Promise(ok => { const r = db.transaction("lib").objectStore("lib").openCursor(); r.onsuccess = () => { const c = r.result; if (c) { out.push(c.value.info); c.continue(); } else ok(); }; r.onerror = ok; }); db.close(); }
    return out.sort((a, b) => a.date - b.date);
  }
  async function removeLibrary(id) {
    if (await probeOpfs()) { const d = await root(); for (const s of [".idx", ".peaks", ".meta.json", ".info.json", ".mzlib", ".tmp"]) { try { await d.removeEntry(id + s); } catch (e) { /* not there */ } } }
    else await idbDo("readwrite", s => s.delete(id));
    LOADED.delete(id);
    const w = WLIBS.get(id); if (w) { try { w.lib.free(); } catch (e) { /* already gone */ } WLIBS.delete(id); rebalance(); }
  }
  const LOADED = new Map();                     // id -> { info, idx, meta, peaks (ArrayBuffer) | file (File) }
  async function loadLibrary(id) {
    if (LOADED.has(id)) return LOADED.get(id);
    let o;
    if (await probeOpfs()) {
      const idx = unpackIndex(await (await readFile(id + ".idx")).arrayBuffer()), meta = JSON.parse(await (await readFile(id + ".meta.json")).text()), info = JSON.parse(await (await readFile(id + ".info.json")).text()), pf = await readFile(id + ".peaks");
      o = { info, idx, meta, file: pf.size > 200e6 ? pf : null, peaks: pf.size > 200e6 ? null : await pf.arrayBuffer() };
    } else {
      const r = await idbDo("readonly", s => s.get(id)); if (!r) throw Object.assign(new Error("library not found"), { key: "lib.err.notFound" });
      o = { info: r.info, idx: unpackIndex(r.idx), meta: r.meta, peaks: r.peaks, file: null };
    }
    LOADED.set(id, o); return o;
  }
  async function peaksOf(lib, pos) {            // the peaks of the spectrum at `pos` (file order)
    const a = lib.idx.off[pos], b = lib.idx.off[pos + 1]; let buf;
    if (lib.peaks) buf = lib.peaks.slice(a * 8, b * 8);
    else {
      if (!lib.pall && lib.file.size < 150e6) lib.pall = await lib.file.arrayBuffer();       // a library up to 150 MB is read once: a search touches thousands of its spectra
      buf = lib.pall ? lib.pall.slice(a * 8, b * 8) : await lib.file.slice(a * 8, b * 8).arrayBuffer();
    }
    const f = new Float32Array(buf), mz = new Float32Array(b - a), it = new Float32Array(b - a);
    for (let i = 0; i < b - a; i++) { mz[i] = f[2 * i]; it[i] = f[2 * i + 1]; }
    return { mz, it };
  }

  async function addLibraryJs(file, reqId) {
    const head = await file.slice(0, 4096).text(), fmt = formatOf(file.name, head);
    if (fmt === "unsupported") throw Object.assign(new Error("unsupported format: export the library as MSP"), { key: "lib.err.unsupported" });
    if (fmt === "unknown") throw Object.assign(new Error("unknown format: MSP or MGF needed"), { key: "lib.err.unknown" });
    const id = "lib" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6), sink = new Sink(id); await sink.open();
    const prec = [], pol = [], off = [0], meta = []; let total = 0, last = 0;
    const parser = new LibParser(fmt, s => { prec.push(s.prec); pol.push(s.pol); meta.push(s.meta); sink.add(s.mz, s.it); total += s.mz.length; off.push(total); });
    const rd = file.stream().getReader(), dec = new TextDecoder("utf-8"); let read = 0;
    for (;;) {
      const { done, value } = await rd.read(); if (done) break;
      read += value.length; parser.push(dec.decode(value, { stream: true }));
      if (Date.now() - last > 150) { last = Date.now(); postMessage({ type: "progress", reqId, read, total: file.size, n: prec.length }); }
    }
    parser.push(dec.decode()); parser.end();
    const peaksBuf = await sink.close();
    if (!prec.length) { if (hasOpfs() && opfsOk) { try { await (await root()).removeEntry(id + ".peaks"); } catch (e) { /* none */ } } throw Object.assign(new Error(`no usable spectrum (dropped: ${parser.stats.dropped})`), { key: "lib.err.noSpectrum", params: { dropped: parser.stats.dropped } }); }
    const idx = buildIndex(prec, pol, off), info = { id, name: file.name.replace(/\.(msp|mgf)$/i, ""), file: file.name, size: file.size, n: prec.length, pos: pol.filter(p => p > 0).length, neg: pol.filter(p => p < 0).length,
      dropped: parser.stats.dropped, noPrec: parser.stats.noPrec, noPeaks: parser.stats.noPeaks, broken: parser.stats.broken, peaks: total, date: Date.now(), fmt };
    await saveLibrary(id, info, packIndex(idx, total), meta, peaksBuf);
    return info;
  }

  async function runSearchJs(m) {
    const ids = m.ids && m.ids.length ? m.ids : (await listLibraries()).map(i => i.id), q = queryOf(m.peaks, m.prec, m.pol), out = [];
    if (!q) return { results: [], libs: ids.length };
    for (const id of ids) {
      const lib = await loadLibrary(id), cache = new Map();
      // the peaks are read before the (synchronous) search: only the candidates of the precursor window
      const tol = m.tol.unit === "ppm" ? q.prec * m.tol.v * 1e-6 : m.tol.v, lo = lowerBound(lib.idx.prec, q.prec - tol);
      for (let k = lo; k < lib.idx.n && lib.idx.prec[k] <= q.prec + tol; k++) { const pos = lib.idx.ord[k]; cache.set(pos, await peaksOf(lib, pos)); }
      const rs = searchIndex(lib.idx, q, { ppm: m.tol.unit === "ppm" ? m.tol.v : null, da: m.tol.unit === "ppm" ? null : m.tol.v, frag: m.frag, pol: m.pol }, pos => cache.get(pos), lib.meta);
      rs.forEach(r => { r.lib = lib.info.name; out.push(r); });
    }
    out.sort((a, b) => b.ent - a.ent || b.cos - a.cos);
    return { results: out.slice(0, 50), libs: ids.length, query: { mz: Array.from(q.mz), it: Array.from(q.it) } };
  }
  async function runAnalogsJs(m) {                 // analogues of one MS2 spectrum in every library (H9): the 10 best by modified cosine
    const ids = m.ids && m.ids.length ? m.ids : (await listLibraries()).map(i => i.id), q = queryOf(m.peaks, m.prec, m.pol), out = [];
    if (!q) return { results: [], libs: ids.length };
    const maxD = m.maxDelta ?? 200;
    for (const id of ids) {
      const lib = await loadLibrary(id), cache = new Map(), lo = lowerBound(lib.idx.prec, q.prec - maxD);
      for (let k = lo; k < lib.idx.n && lib.idx.prec[k] <= q.prec + maxD; k++) { const pos = lib.idx.ord[k]; const lp = lib.idx.pol[pos]; if (m.pol && lp && lp !== m.pol) continue; cache.set(pos, await peaksOf(lib, pos)); }
      analogIndex(lib.idx, q, { maxDelta: maxD, frag: m.frag, pol: m.pol }, pos => cache.get(pos), lib.meta).forEach(r => { r.lib = lib.info.name; out.push(r); });
    }
    out.sort((a, b) => b.mcos - a.mcos);
    return { results: out.slice(0, m.max || 10), libs: ids.length, query: { mz: Array.from(q.mz), it: Array.from(q.it) } };
  }
  const ikBlock = r => { const k = String(r.ik || "").slice(0, 14).toUpperCase(); return k || "n:" + String(r.name || "").toLowerCase().replace(/[^a-z0-9]+/g, ""); };       // first InChIKey block (or the name): one structure
  const lightOf = r => ({ name: r.name, ik: r.ik || "", ent: r.ent, cos: r.cos, shared: r.shared, dprec: r.dprec, lib: r.lib, nlib: r.nlib });
  function withAlt(cands) {                       // the best candidate and, in `alt`, the best one of ANOTHER structure: the lead is the «distinct» gate of the confidence levels (livelli.js)
    const l = cands.filter(Boolean).sort((a, b) => b.ent - a.ent); if (!l.length) return null;
    const k = ikBlock(l[0]), a = l.find(x => ikBlock(x) !== k); return { ...l[0], alt: a ? lightOf(a) : null };
  }
  const rowOf = (Q, best) => ({ key: Q.key, rt: Q.rt, prec: Q.prec, name: best ? best.name : "", formula: best ? best.formula : "", ik: best ? best.ik || "" : "", dprec: best ? best.dprec : null, lib: best ? best.lib : "", ent: best ? best.ent : null, cos: best ? best.cos : null, shared: best ? best.shared : null, nlib: best ? best.nlib : null, alt: best ? best.alt || null : null,
    accession: best ? best.accession || "" : "", authors: best ? best.authors || "" : "", license: best ? best.license || "" : "", smiles: best ? best.smiles || "" : "", ce: best ? best.ce || "" : "", inst: best ? best.inst || "" : "", type: best ? best.type || "" : "" });      // record, authors and licence of the best entry: «Identify all» and the untargeted tables cite them
  async function runAllJs(m, reqId) {             // every MS2 of a file: the best result of each (the query list comes from the page)
    const ids = m.ids && m.ids.length ? m.ids : (await listLibraries()).map(i => i.id), libs = []; for (const id of ids) libs.push(await loadLibrary(id));
    const rows = []; let last = 0;
    for (let n = 0; n < m.queries.length; n++) {
      const Q = m.queries[n], q = queryOf(Q.peaks, Q.prec, Q.pol), found = [];
      if (q) for (const lib of libs) {
        const tol = m.tol.unit === "ppm" ? q.prec * m.tol.v * 1e-6 : m.tol.v, lo = lowerBound(lib.idx.prec, q.prec - tol), cache = new Map();
        for (let k = lo; k < lib.idx.n && lib.idx.prec[k] <= q.prec + tol; k++) { const pos = lib.idx.ord[k]; cache.set(pos, await peaksOf(lib, pos)); }
        for (const r of searchIndex(lib.idx, q, { ppm: m.tol.unit === "ppm" ? m.tol.v : null, da: m.tol.unit === "ppm" ? null : m.tol.v, frag: m.frag, pol: Q.pol }, pos => cache.get(pos), lib.meta)) found.push({ ...r, lib: lib.info.name, nlib: r.mz ? r.mz.length : undefined });
      }
      rows.push(rowOf(Q, withAlt(found)));
      if (Date.now() - last > 200) { last = Date.now(); postMessage({ type: "progress", reqId, read: n + 1, total: m.queries.length }); }
    }
    return { rows };
  }


  // ---------------------------------------------------------------- the WebAssembly engine (.mzlib libraries)
  let W = null, wasmTried = false, forceJs = false, allowWasm = false, migrated = false;
  const MZLIB_ERRORS = ["lib.err.corrupt", "lib.err.version", "lib.err.tooLarge", "lib.err.io", "lib.err.needWasm", "error.memoryBudget"];     // keys the engine can answer with (the page translates them)
  async function wasm() {                       // null where WebAssembly or the file is not there (old browser, local development server): the JavaScript path stays
    if (forceJs || !allowWasm) return null;
    if (wasmTried) return W;
    wasmTried = true;
    try { if (typeof WebAssembly !== "object") throw new Error("no WebAssembly"); const mod = await import("./wasm-lib/mzlab_lib_wasm.js"); await mod.default(); W = mod; } catch (e) { W = null; }
    return W;
  }
  const smallDevice = () => { const ua = (self.navigator && navigator.userAgent) || ""; return /iPad|iPhone|Android|Mobile/i.test(ua) || (/Macintosh/.test(ua) && (navigator.maxTouchPoints || 0) > 1); };
  const isW = info => !!(info && info.store === "mzlib");
  const blobReader = blob => { const fr = new FileReaderSync(); return (off, len) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(off, off + len))); };
  const WLIBS = new Map();                      // id -> { lib (wasm Lib), info }
  function rebalance() { if (!W || !WLIBS.size) return; const per = W.total_cache_budget(smallDevice()) / WLIBS.size; for (const e of WLIBS.values()) e.lib.set_cache_budget(per); }
  async function mzlibBlob(id) {
    if (CARTELLA.isFolderId(id)) { const v = FOLD.map.get(id); if (!v) throw Object.assign(new Error("library not found"), { key: "lib.err.notFound" }); return FOLD.dir.file(CARTELLA.blobPath(v)); }
    if (await probeOpfs()) return readFile(id + ".mzlib");
    const r = await idbDo("readonly", s => s.get(id));
    if (!r || !r.mzlib) throw Object.assign(new Error("library not found"), { key: "lib.err.notFound" });
    return r.mzlib;
  }
  async function wlib(id) {
    const have = WLIBS.get(id); if (have) return have.lib;
    const w = await wasm(); if (!w) throw Object.assign(new Error("the fast engine is not available"), { key: "lib.err.needWasm" });
    const blob = await mzlibBlob(id), lib = w.Lib.open(CARTELLA.isFolderId(id) ? FOLD.dir.reader(blob) : blobReader(blob), blob.size, w.total_cache_budget(smallDevice()) / (WLIBS.size + 1));
    WLIBS.set(id, { lib }); rebalance();
    return lib;
  }
  // a file the engine reads and writes by offset: an OPFS synchronous handle, or memory (browsers without OPFS)
  async function openSyncFile(name) {
    const h = await (await (await root()).getFileHandle(name, { create: true })).createSyncAccessHandle(); h.truncate(0);
    return { read: (off, len) => { const b = new Uint8Array(len); h.read(b, { at: off }); return b; }, write: (off, data) => { if (h.write(data, { at: off }) !== data.length) throw new Error("storage full"); }, size: () => h.getSize(),
      async close(remove) { try { h.flush(); h.close(); } catch (e) { /* closed */ } if (remove) { try { await (await root()).removeEntry(name); } catch (e) { /* not there */ } } } };
  }
  function memFile() {
    const CH = 1 << 22, chunks = []; let size = 0;
    return { read(off, len) { const o = new Uint8Array(len); let p = 0; while (p < len) { const ci = Math.floor((off + p) / CH), co = (off + p) % CH, n = Math.min(len - p, CH - co), c = chunks[ci]; if (c) o.set(c.subarray(co, co + n), p); p += n; } return o; },
      write(off, data) { let p = 0; while (p < data.length) { const ci = Math.floor((off + p) / CH), co = (off + p) % CH, n = Math.min(data.length - p, CH - co); (chunks[ci] || (chunks[ci] = new Uint8Array(CH))).set(data.subarray(p, p + n), co); p += n; } size = Math.max(size, off + data.length); },
      size: () => size, async close() { /* memory */ }, blob() { const parts = chunks.map((c, i) => (i === chunks.length - 1 ? c.subarray(0, size - i * CH) : c)); return new Blob(parts, { type: "application/octet-stream" }); } };
  }
  async function saveMzlib(id, info, out) {
    if (opfsOk) await writeSmall(id + ".info.json", JSON.stringify(info));
    else await idbDo("readwrite", s => s.put({ info, mzlib: out.blob() }, id));
  }
  // reads `file` (a File, or a file of the native folder) into a .mzlib: scratch space and output in OPFS, or in memory where the browser has no synchronous handles
  async function runImport(w, file, reqId, onChunk) {
    const head = new Uint8Array(await file.slice(0, 8192).arrayBuffer()), id = "lib" + Date.now().toString(36) + Math.random().toString(36).slice(2, 6), opfs = await probeOpfs();
    const scratch = opfs ? await openSyncFile(id + ".tmp") : memFile(), out = opfs ? await openSyncFile(id + ".mzlib") : memFile();
    let imp = null;
    try {
      imp = new w.Importer(file.name, head, scratch.read, scratch.write);
      const rd = file.stream().getReader(); let read = 0, last = 0;
      for (;;) {
        const { done, value } = await rd.read(); if (done) break;
        read += value.length; imp.push(value); if (onChunk) onChunk(value);
        if (Date.now() - last > 150) { last = Date.now(); postMessage({ type: "progress", reqId, read, total: file.size, n: imp.count() }); }
      }
      const r = JSON.parse(imp.finish(out.write)); imp.free(); imp = null;
      await scratch.close(true); await out.close(false);
      return { r, id, out, opfs };
    } catch (e) {
      try { if (imp) imp.free(); } catch (e2) { /* gone */ }
      await scratch.close(true); await out.close(true);
      throw e;
    }
  }
  const infoOfImport = (id, file, r) => ({ id, name: file.name.replace(/\.(msp|mgf|json|jsonl|ndjson|txt)$/i, ""), file: file.name, size: file.size, n: r.n, pos: r.pos, neg: r.neg, dropped: r.dropped, noPrec: r.noPrec, noPeaks: r.noPeaks, broken: r.broken, peaks: r.peaks, date: Date.now(), store: "mzlib", bytes: r.bytes });
  async function addLibraryWasm(w, file, reqId) {
    const { r, id, out } = await runImport(w, file, reqId), info = infoOfImport(id, file, r);
    await saveMzlib(id, info, out);
    return info;
  }

  // ---------------------------------------------------------------- the folder of the libraries (libreria-cartella.js)
  const FOLD = { dir: null, mode: null, map: new Map(), infos: [], cat: null, tried: false, catTried: false };
  async function setFolder(m) {                     // {mode: "opfs" | "fs" (handle) | "native" (base of the mzlab:// protocol)}
    FOLD.tried = true; FOLD.map = new Map(); FOLD.infos = []; FOLD.dir = null; FOLD.mode = null;
    for (const [id, e] of [...WLIBS]) if (CARTELLA.isFolderId(id)) { try { e.lib.free(); } catch (e2) { /* gone */ } WLIBS.delete(id); }
    if (m.mode === "native") { FOLD.dir = CARTELLA.nativeDir(m.base); try { FOLD.dir.name = (await (await fetch(m.base + "lib/dir")).json()).dir; } catch (e) { /* the name is only shown */ } }
    else if (m.mode === "fs" && m.handle) FOLD.dir = CARTELLA.handleDir(m.handle, "fs");
    else if (hasOpfs() && (await probeOpfs())) FOLD.dir = CARTELLA.handleDir(await (await root()).getDirectoryHandle("librerie", { create: true }), "opfs");
    FOLD.mode = FOLD.dir && FOLD.dir.kind;
    if (m.cat) FOLD.cat = m.cat;
  }
  async function foldRefresh() {                    // the inventory of the folder (a SHA-256 only for the files that changed)
    if (!FOLD.tried) await setFolder({ mode: "opfs" });
    if (!FOLD.dir) return { voci: [] };
    if (!FOLD.cat && !FOLD.catTried) { FOLD.catTried = true; try { FOLD.cat = SCARICA.parseCatalog(await (await FOLD.dir.file(CARTELLA.META + "/catalogo.json")).text()); } catch (e) { /* no catalog seen yet */ } }   // the one the user fetched last time: «update available» without a network call
    const s = await CARTELLA.scan(FOLD.dir, FOLD.cat);
    FOLD.map = new Map(s.voci.map(v => [v.id, v]));
    FOLD.infos = s.voci.filter(v => (v.stato === "pronta" || v.stato === "aggiornamento") && v.info).map(v => ({ ...v.info, id: v.id, name: CARTELLA.stem(v.nome), file: v.nome, size: v.tipo === "pacchetto" ? v.size : (v.info.bytes || 0),
      date: v.info.date || v.mtime, store: "mzlib", cartella: true, tipo: v.tipo, stato: v.stato })).sort((a, b) => a.date - b.date || a.name.localeCompare(b.name));
    return s;
  }
  async function everyLibrary() {
    const legacy = await listLibraries();
    if (!FOLD.tried || !FOLD.map.size) { try { await foldRefresh(); } catch (e) { /* a folder that cannot be read has no libraries */ } }
    return [...legacy, ...FOLD.infos];
  }
  const foldName = () => (FOLD.dir ? FOLD.dir.name || "" : "");
  async function foldAdd(w, file, reqId) {          // index a file (of the folder or imported) into `.mzlab/` of the folder: the index first, the meta file last (it marks the entry as done)
    const key = file.name, sha = new CARTELLA.Sha256(), M = CARTELLA.META;
    if (/\.mzlib$/i.test(key)) { await FOLD.dir.write(key, file); await foldRefresh(); return { id: CARTELLA.idOf(key), name: CARTELLA.stem(key), n: 0, dropped: 0 }; }
    const { r, id, out, opfs } = await runImport(w, file, reqId, v => sha.update(v));
    const blob = opfs ? await readFile(id + ".mzlib") : out.blob();
    try {
      await FOLD.dir.write(M + "/" + key + ".mzlib.tmp", blob); await FOLD.dir.replace(M + "/" + key + ".mzlib.tmp", M + "/" + key + ".mzlib");
      const info = infoOfImport(CARTELLA.idOf(key), file, r);
      await FOLD.dir.write(M + "/" + key + ".idx.json", JSON.stringify(CARTELLA.metaOf(key, file.size, file.lastModified || 0, sha.hex(), info, blob.size)));
      await foldRefresh();
      return info;
    } finally { if (opfs) { try { await (await root()).removeEntry(id + ".mzlib"); } catch (e) { /* not there */ } } }
  }
  // the catalog of the packages: from the site (only when the user asks) or handed over by the app; kept in the folder for the states of the next visit
  async function catalogCmd(m) {
    if (!FOLD.dir) throw Object.assign(new Error("no folder"), { key: "lib.dl.err.io", params: { detail: "folder" } });
    if (!(m.cached && FOLD.cat)) {
      let text;
      if (m.cat) text = JSON.stringify(m.cat);
      else {
        let res; try { res = await fetch("librerie/catalogo.json", { cache: "no-store" }); } catch (e) { throw Object.assign(new Error("offline"), { key: "lib.dl.err.offline", params: { detail: String(e && e.message || e) } }); }
        if (res.status === 404) throw Object.assign(new Error("no catalog"), { key: "lib.dl.err.nocatalog" });
        if (!res.ok) throw Object.assign(new Error("http"), { key: "lib.dl.err.http", params: { status: res.status } });
        text = await res.text();
      }
      FOLD.cat = SCARICA.parseCatalog(text); FOLD.catTried = true;
      try { await FOLD.dir.write(CARTELLA.META + "/catalogo.json", text); } catch (e) { /* the states of the next visit only */ }
    }
    const s = await foldRefresh();
    return { cat: FOLD.cat, avail: FOLD.cat.librerie.map(p => ({ id: p.id, stato: SCARICA.stateOf(p, s.voci) })) };
  }
  const DL = { abort: null };
  async function downloadCmd(m, reqId) {            // the packages the user ticked, one after the other; a failure of one does not stop the others (a cancel does)
    if (!FOLD.dir || !FOLD.cat) throw Object.assign(new Error("no catalog"), { key: "lib.dl.err.nocatalog" });
    DL.abort = new AbortController(); const done = [], failed = [], base = new URL("librerie/", self.location.href).href;
    try {
      for (const id of m.ids) {
        const pkg = FOLD.cat.librerie.find(p => p.id === id);
        if (!pkg) { failed.push({ id, key: "lib.dl.err.catalog", params: { detail: id } }); continue; }
        try {
          await SCARICA.downloadPackage({ dir: FOLD.dir, pkg, base, signal: DL.abort.signal, onProgress: p => postMessage({ type: "progress", reqId, ...p }),
            beforeReplace: async () => { const w = WLIBS.get(CARTELLA.idOf(id + ".mzlib")); if (w) { try { w.lib.free(); } catch (e) { /* gone */ } WLIBS.delete(CARTELLA.idOf(id + ".mzlib")); rebalance(); } } });
          done.push(id);
        } catch (e) {
          failed.push({ id, key: e.key || "lib.dl.err.io", params: e.params || { detail: String(e && e.message || e) } });
          if (e.key === "lib.dl.err.cancelled") break;
        }
      }
    } finally { DL.abort = null; }
    const s = await foldRefresh();
    return { done, failed, avail: FOLD.cat.librerie.map(p => ({ id: p.id, stato: SCARICA.stateOf(p, s.voci) })) };
  }
  async function foldRemove(id) {                   // a source stays in the folder (only its index goes); a package or an imported index is removed
    const v = FOLD.map.get(id); if (!v) return; const M = CARTELLA.META;
    if (v.tipo === "pacchetto") { await FOLD.dir.remove(v.nome); await FOLD.dir.remove(M + "/" + v.nome + ".pkg.json"); }
    else { await FOLD.dir.remove(M + "/" + v.nome + ".idx.json"); await FOLD.dir.remove(M + "/" + v.nome + ".mzlib"); }
    const w = WLIBS.get(id); if (w) { try { w.lib.free(); } catch (e) { /* gone */ } WLIBS.delete(id); rebalance(); }
    await foldRefresh();
  }
  async function addLibrary(file, reqId) {
    const w = await wasm();
    if (w) { if (!FOLD.tried) await setFolder({ mode: "opfs" }); if (FOLD.dir) return foldAdd(w, file, reqId); return addLibraryWasm(w, file, reqId); }
    return addLibraryJs(file, reqId);
  }
  // an old MZL1 archive (top 100 peaks as Float32 pairs) becomes a .mzlib once; then the old files go
  async function migrateOne(w, info) {
    const id = info.id, opfs = await probeOpfs(); let idx, meta, peaks, file = null;
    if (opfs) { idx = unpackIndex(await (await readFile(id + ".idx")).arrayBuffer()); meta = JSON.parse(await (await readFile(id + ".meta.json")).text()); file = await readFile(id + ".peaks"); peaks = file.size > 200e6 ? null : await file.arrayBuffer(); }
    else { const r = await idbDo("readonly", s => s.get(id)); idx = unpackIndex(r.idx); meta = r.meta; peaks = r.peaks; }
    const n = idx.n, precAt = new Float64Array(n); for (let k = 0; k < n; k++) precAt[idx.ord[k]] = idx.prec[k];
    const scratch = opfs ? await openSyncFile(id + ".tmp") : memFile(), out = opfs ? await openSyncFile(id + ".mzlib") : memFile(), rd = file && !peaks ? blobReader(file) : null;
    let imp = null;
    try {
      imp = w.Importer.forSpectra(scratch.read, scratch.write);
      for (let i = 0; i < n; i++) {
        const a = idx.off[i], b = idx.off[i + 1], f = new Float32Array(peaks ? peaks.slice(a * 8, b * 8) : rd(a * 8, (b - a) * 8).buffer), mz = new Float64Array(b - a), it = new Float64Array(b - a);
        for (let j = 0; j < b - a; j++) { mz[j] = f[2 * j]; it[j] = f[2 * j + 1]; }
        imp.addSpectrum(precAt[i], idx.pol[i], mz, it, JSON.stringify(meta[i] || []));
      }
      const r = JSON.parse(imp.finish(out.write)); imp.free(); imp = null;
      await scratch.close(true); await out.close(false);
      const ni = { ...info, n: r.n, pos: r.pos, neg: r.neg, dropped: (info.dropped || 0) + (n - r.n), peaks: r.peaks, store: "mzlib", bytes: r.bytes, fmt: undefined };
      await saveMzlib(id, ni, out);
      if (opfs) { const d = await root(); for (const sfx of [".idx", ".peaks", ".meta.json"]) { try { await d.removeEntry(id + sfx); } catch (e) { /* not there */ } } }
      LOADED.delete(id);
    } catch (e) { try { if (imp) imp.free(); } catch (e2) { /* gone */ } await scratch.close(true); await out.close(true); throw e; }
  }
  async function migrateAll() {
    if (migrated) return; const w = await wasm(); if (!w) return; migrated = true;
    for (const info of await listLibraries()) if (!isW(info)) { try { await migrateOne(w, info); } catch (e) { /* the old archive stays and is searched by the JavaScript path */ } }
  }
  const ppmOf = t => t.unit === "ppm";
  const arrays = peaks => { const p = peaks.filter(x => x[1] > 0 && x[0] > 0); return [Float64Array.from(p, x => x[0]), Float64Array.from(p, x => x[1])]; };
  function remapPairs(r, from, to, tol) {         // the pairs of a result computed against another cleaned query: indices into `to` by m/z
    const at = i => { let best = -1, bd = tol; for (let j = 0; j < to.mz.length; j++) { const d = Math.abs(to.mz[j] - from.mz[i]); if (d <= bd) { bd = d; best = j; } } return best; };
    r.pairs = (r.pairs || []).map(p => [at(p[0]), p[1]]).filter(p => p[0] >= 0);
  }
  async function selection(m) {
    const infos = await everyLibrary(), byId = new Map(infos.map(i => [i.id, i])), ids = m.ids && m.ids.length ? m.ids : infos.map(i => i.id);
    return { ids, wIds: ids.filter(id => isW(byId.get(id))), jIds: ids.filter(id => !isW(byId.get(id))), byId };
  }
  async function runSearch(m) {
    const { ids, wIds, jIds, byId } = await selection(m);
    if (!wIds.length) return runSearchJs(m);
    const w = await wasm(); if (!w) throw Object.assign(new Error("the fast engine is not available"), { key: "lib.err.needWasm" });
    const [mz, it] = arrays(m.peaks), out = []; let query = null;
    for (const id of wIds) {
      const lib = await wlib(id), r = JSON.parse(lib.identity(m.prec, m.pol, mz, it, m.tol.v, ppmOf(m.tol), m.frag, 50));
      if (!query && r.query.mz.length) query = r.query;
      r.results.forEach(x => { x.lib = byId.get(id).name; out.push(x); });
    }
    if (jIds.length) { const j = await runSearchJs({ ...m, ids: jIds }); j.results.forEach(x => { if (query) remapPairs(x, j.query, query, m.frag); out.push(x); }); if (!query) query = j.query; }
    out.sort((a, b) => b.ent - a.ent || b.cos - a.cos);
    return { results: out.slice(0, 50), libs: ids.length, query: query || { mz: [], it: [] } };
  }
  async function runAnalogs(m) {
    const { ids, wIds, jIds, byId } = await selection(m);
    if (!wIds.length) return runAnalogsJs(m);
    const w = await wasm(); if (!w) throw Object.assign(new Error("the fast engine is not available"), { key: "lib.err.needWasm" });
    const [mz, it] = arrays(m.peaks), out = [], top = m.max || 10; let query = null;
    for (const id of wIds) {
      const lib = await wlib(id), r = JSON.parse(lib.analogs(m.prec, m.pol, mz, it, m.frag, m.maxDelta ?? 200, top));
      if (!query && r.query.mz.length) query = r.query;
      r.results.forEach(x => { x.lib = byId.get(id).name; out.push(x); });
    }
    if (jIds.length) (await runAnalogsJs({ ...m, ids: jIds })).results.forEach(x => out.push(x));
    out.sort((a, b) => b.mcos - a.mcos);
    return { results: out.slice(0, top), libs: ids.length, query: query || { mz: [], it: [] } };
  }
  async function runAll(m, reqId) {
    const { ids, wIds, jIds, byId } = await selection(m);
    if (!wIds.length) return runAllJs(m, reqId);
    const w = await wasm(); if (!w) throw Object.assign(new Error("the fast engine is not available"), { key: "lib.err.needWasm" });
    const libs = []; for (const id of wIds) libs.push({ lib: await wlib(id), name: byId.get(id).name });
    const legacy = jIds.length ? (await runAllJs({ ...m, ids: jIds }, reqId)).rows : null, rows = []; let last = 0;
    for (let n = 0; n < m.queries.length; n++) {
      const Q = m.queries[n], [mz, it] = arrays(Q.peaks), found = [];
      for (const L of libs) {
        const r = JSON.parse(L.lib.best(Q.prec, Q.pol, mz, it, m.tol.v, ppmOf(m.tol), m.frag));
        if (r) { found.push({ ...r, lib: L.name }); if (r.alt) found.push({ ...r.alt, lib: L.name }); }
      }
      let row = rowOf(Q, withAlt(found));
      const lr = legacy && legacy[n]; if (lr && lr.ent != null && (row.ent == null || lr.ent > row.ent)) row = lr;
      rows.push(row);
      if (Date.now() - last > 200) { last = Date.now(); postMessage({ type: "progress", reqId, read: n + 1, total: m.queries.length }); }
    }
    return { rows };
  }

  // one handler per command: (message, request id) -> the fields of the answer
  async function listCmd() {
    await migrateAll();
    let voci = [], r = {};
    if (await wasm()) { try { voci = (await foldRefresh()).voci; } catch (e) { r = { folderError: String(e && e.name || e) }; } }
    return { ...r, libs: [...(await listLibraries()), ...FOLD.infos], voci, folder: { mode: FOLD.mode, name: foldName() }, opfs: await probeOpfs() };
  }
  async function indexCmd(m, reqId) {
    const w = await wasm();
    if (!w || !FOLD.dir) throw Object.assign(new Error("the fast engine is not available"), { key: "lib.err.needWasm" });
    return { info: await foldAdd(w, await FOLD.dir.file(m.name), reqId) };
  }
  const COMMANDS = {
    config: m => { forceJs = !!m.forceJs; allowWasm = !!m.wasm; return {}; },
    engine: async () => ({ wasm: !!(await wasm()) }),
    crash: async () => { const w = await wasm(); if (w) w.crash(); return {}; },
    folder: async () => { await FOLD.pending; return {}; },
    catalog: m => catalogCmd(m),
    download: (m, reqId) => downloadCmd(m, reqId),
    cancel: () => { if (DL.abort) DL.abort.abort(); return {}; },
    list: listCmd,
    add: async (m, reqId) => ({ info: await addLibrary(m.file, reqId) }),
    index: indexCmd,
    remove: async m => { if (CARTELLA.isFolderId(m.id)) await foldRemove(m.id); else await removeLibrary(m.id); return {}; },
    search: m => runSearch(m),
    searchAll: (m, reqId) => runAll(m, reqId),
    analogs: m => runAnalogs(m),
  };

  onmessage = async e => {
    const m = e.data, reqId = m.reqId;
    try {
      if (m.cmd === "folder") { FOLD.tried = true; FOLD.pending = (FOLD.pending || Promise.resolve()).then(() => setFolder(m)); }      // in order, and set at once: the messages that follow wait for it
      else if (m.cmd !== "config" && FOLD.pending) await FOLD.pending;
      const run = Object.hasOwn(COMMANDS, m.cmd) ? COMMANDS[m.cmd] : null;
      if (!run) throw new Error("comando sconosciuto: " + m.cmd);
      const r = await run(m, reqId);
      postMessage({ type: "done", reqId, ...r });
    } catch (err) {
      let j = null; if (typeof err === "string" && err.startsWith("{")) { try { j = JSON.parse(err); } catch (e2) { j = null; } }      // an error of the engine itself: {"error_key", "params"}
      const fatal = !j && !!W && (err instanceof WebAssembly.RuntimeError || (err instanceof RangeError && /memory/i.test(String(err.message))));      // a trap or a lost memory: the instance is gone, the page starts a new worker
      postMessage({ type: "error", reqId, fatal, error: j ? j.error_key : String(err && err.message || err), error_key: j ? j.error_key : err && err.key, params: j ? j.params : err && err.params });
    }
  };
}
