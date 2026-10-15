// @ts-check
// Files of a maker (Thermo .raw, SCIEX .wiff + .wiff.scan, Agilent .D folder) read as they are, in the engine's worker (rust-worker.js): no mzML is written. The members of a dataset
// are kept in OPFS `files/` under flat names; a sample is a «file» of the program called `x.mzML` whose bytes are a small JSON (the stub: which members, which sample), so the list of the
// files, the index (`idx/x.mzML.mzidx`) and the removal work as for any mzML. The engine reads the members through their offsets (crates/mzlab-wasm/src/nativi.rs).
import { putChunks, ensureRoom } from "./archivio.js";
import { rawSink } from "./rust-opfs.js";
import { W } from "./nativo-fasi.js";
import { createEsterno } from "./lettore-esterno.js";

export const MAGIC = "mzlab/nat1";
export const UPLOAD_ERROR = "err.file.unknown";              // the key the desktop engine gives for a file that did not reach it (the engine is in crates/mzlab-desktop)
export const PREFIX = "nat~";                              // the members of a dataset in files/: nat~<key>~<path with ~ for />
const TICK = 200;                                         // ms between two progress messages
const PHASES = ["read", "read", "scans", "chrom"];        // the name of the phase shown, by the phase number of build_mzidx_native (0 = the copy)

/** @typedef {{ name: string, store: string, size: number }} StubMember */
/** @typedef {{ mzlab: string, ds: string, c: number, id: number, centroid: boolean, members: StubMember[], size: number }} Stub */

/** the stub in a File of the archive, or null when it is an ordinary file (an mzML, a member) */
export async function readStub(/** @type {Blob} */ f) {
  if (!f || f.size > 65536 || f.size < 20) return null;
  try { const j = JSON.parse(await f.text()); return j && j.mzlab === MAGIC ? /** @type {Stub} */ (j) : null; } catch (_) { return null; }
}

/** `read(member, offset, length)` over some Files, as nativi.rs wants it; `seen(i, end)` is told how far each member was read */
export function reader(/** @type {Blob[]} */ files, /** @type {((i: number, end: number) => void) | undefined} */ seen) {
  const fr = new FileReaderSync(), rd = files.map(f => (/** @type {number} */ o, /** @type {number} */ n) => new Uint8Array(fr.readAsArrayBuffer(f.slice(o, o + n))));
  return (/** @type {number} */ i, /** @type {number} */ o, /** @type {number} */ n) => { const b = rd[i](o, n); if (seen) seen(i, o + b.length); return b; };
}

/** runs in the index builder (idx-worker.js): builds the .mzidx of the sample with `build_mzidx_native` of the .wasm (given, so this module has no import of it); `write(offset, bytes)` writes it; `progress({frac, phase})` is told the overall fraction 0..1 */
export async function buildNative(/** @type {Function} */ build_mzidx_native, /** @type {{ prepara: (names: string[]) => Promise<void> }} */ ext, /** @type {{ st: Stub, files: Blob[] }} */ j, /** @type {string} */ name, /** @type {(o: number, d: Uint8Array) => void} */ write, /** @type {(p: { frac: number, phase: string }) => void} */ progress, /** @type {number} */ budget) {
  await ext.prepara(j.st.members.map(x => x.name));
  const sizes = j.files.map(f => f.size), all = sizes.reduce((a, b) => a + b, 0) || 1, high = sizes.map(() => 0);
  let phase = 1, last = 0;
  const base = () => W.copy + (phase > 1 ? W.sha : 0) + (phase > 2 ? W.scans : 0);
  const span = () => [W.sha, W.scans, W.chrom][phase - 1];
  const tell = (/** @type {boolean} */ force) => {
    const t = performance.now(); if (!force && t - last < TICK) return; last = t;
    const within = Math.min(1, high.reduce((a, b) => a + b, 0) / all);
    progress({ frac: Math.min(0.999, base() + span() * within), phase: PHASES[phase] });
  };
  const rd = reader(j.files, (i, end) => { if (end > high[i]) high[i] = end; tell(false); });
  const stats = build_mzidx_native(j.st.members.map(x => x.name), Float64Array.from(sizes), rd, j.st.c, name, j.st.centroid, write, budget,
    (/** @type {number} */ p) => { phase = p; high.fill(0); tell(true); });
  return stats;
}
/** the SRM traces of a sample (`srm::impacchetta` bytes; mzFinder integrates the MRM files from them, no mzML is written): `native_srm` gives the bytes once to its sink @param {Stub} st @param {(st: Stub) => Promise<Blob[]>} memberFiles @param {Function} native_srm @param {{ prepara: (names: string[]) => Promise<void> }} ext */
async function srmDi(st, memberFiles, native_srm, ext) {
  const files = await memberFiles(st);
  await ext.prepara(st.members.map(x => x.name));
  /** @type {Uint8Array | null} */ let out = null;
  native_srm(st.members.map(x => x.name), Float64Array.from(files.map(f => f.size)), reader(files, undefined), st.c, st.centroid, (/** @type {Uint8Array} */ b) => { out = b; });
  return out;
}

/** @param {{ ARCH: any, readerFor: (b: Blob) => (o: number, n: number) => Uint8Array, fileOf: (n: string) => Promise<File>, jerr: (k: string) => never, native_samples: Function, native_to_mzml: Function, native_srm: Function, native_external: Function }} h */
export function createNativo({ ARCH, readerFor, fileOf, jerr, native_samples, native_to_mzml, native_srm, native_external }) {
  const ext = createEsterno(native_external);               // a Thermo .raw is read through the `thermo` module (lettore-esterno.js)
  const stubs = new Map();                                // mzML name -> Stub (read from the file once)
  const writing = new Map();                              // store name -> Promise: the samples of a .wiff share their members, written once
  const stubOf = async (/** @type {string} */ name) => stubs.get(name) || stubs.set(name, await readStub(await fileOf(name))).get(name);

  /** the Files of the members of a stub (read from OPFS) */
  async function memberFiles(/** @type {Stub} */ st) {
    const out = [];
    for (const m of st.members) { const f = await ARCH.get("files", m.store); if (!f) jerr("err.session.fileNotFound"); out.push(f); }
    return out;
  }
  /** attaches slot `slot` (whose .mzidx is open) to its reader when `name` is a native sample; true when it is */
  async function attach(/** @type {any} */ engine, /** @type {string} */ name, /** @type {number} */ slot) {
    const st = await stubOf(name);
    if (!st) return false;
    const files = await memberFiles(st);
    await ext.prepara(st.members.map(m => m.name));
    engine.native_attach(slot, st.ds, st.members.map(m => m.name), Float64Array.from(files.map(f => f.size)), reader(files, undefined), st.c, st.centroid);
    return true;
  }

  /** the members of an upload copied to OPFS (once per store name), then the stub; `tell(frac)` 0..1 of the copy */
  async function put(/** @type {any} */ m, /** @type {(f: number) => void} */ tell) {
    const nat = m.nat, total = nat.members.reduce((/** @type {number} */ a, /** @type {any} */ x) => a + x.blob.size, 0);
    await ensureRoom(total);
    let done = 0, last = 0;
    const bump = (/** @type {number} */ n) => { done += n; const t = performance.now(); if (t - last > TICK || done >= total) { last = t; tell(total ? Math.min(1, done / total) : 1); } };
    /** @type {StubMember[]} */ const members = [];
    for (const x of nat.members) {
      const store = PREFIX + nat.key + "~" + x.name.replace(/[\\/]/g, "~");
      members.push({ name: x.name, store, size: x.blob.size });
      const have = await ARCH.get("files", store);
      if (writing.has(store)) { await writing.get(store); bump(x.blob.size); continue; }
      if (have && have.size === x.blob.size) { bump(x.blob.size); continue; }
      const job = putChunks("files", store, x.blob.size, async (/** @type {number} */ o, /** @type {number} */ n) => { const b = new Uint8Array(await x.blob.slice(o, o + n).arrayBuffer()); bump(b.length); return b; });
      writing.set(store, job);
      try { await job; } finally { writing.delete(store); }
    }
    /** @type {Stub} */
    const st = { mzlab: MAGIC, ds: nat.key, c: nat.c, id: nat.id, centroid: !!nat.centroid, members, size: total };
    stubs.delete(m.name);
    return ARCH.put("files", m.name, new TextEncoder().encode(JSON.stringify(st)));
  }

  /** the message of the index builder for a native sample (idx-worker.js); progress comes back as { frac, phase } */
  async function indexJob(/** @type {string} */ name) {
    const st = await stubOf(name);
    if (!st) return null;
    return { st, files: await memberFiles(st) };
  }

  /** the members that no sample points to any more go (`keep`: the names of the stubs left) */
  async function sweep(/** @type {Iterable<string>} */ keep) {
    const used = new Set();
    for (const n of keep) { const st = await stubOf(n); if (st) st.members.forEach(m => used.add(m.store)); }
    for (const e of await ARCH.list("files")) if (e.name.startsWith(PREFIX) && !used.has(e.name)) await ARCH.remove("files", e.name);
  }

  /** «Scarica .mzML»: the mzML of a sample written to a file of OPFS (`dest` in conv/) */
  async function toMzml(/** @type {any} */ m, /** @type {(p: any) => void} */ post) {
    const st = await stubOf(m.name);
    if (!st) jerr("err.session.fileNotFound");
    const files = await memberFiles(/** @type {Stub} */ (st));
    await ext.prepara(/** @type {Stub} */ (st).members.map(x => x.name));
    const sink = await rawSink(m.dest);
    let written = 0, last = 0, scans = 0, scansTotal = null;
    const tick = (/** @type {boolean} */ force) => { const t = performance.now(); if (force || t - last > TICK) { last = t; post({ written, scans, scansTotal }); } };
    try {
      native_to_mzml(/** @type {Stub} */ (st).members.map(x => x.name), Float64Array.from(files.map(f => f.size)), reader(files, undefined), /** @type {Stub} */ (st).c, /** @type {Stub} */ (st).centroid,
        (/** @type {Uint8Array} */ piece) => { sink.write(piece); written += piece.length; tick(false); },
        (/** @type {number} */ d, /** @type {number} */ t) => { scans = d; scansTotal = t; tick(d === 0 || d === t); });
    } catch (e) { const q = await sink.error(); await sink.abort(); throw q || e; }
    return sink.finish();
  }

  /** the samples of the dataset a page describes: JSON `{format, samples: [{id, name, scans}]}` */
  async function samples(/** @type {{ name: string, blob: Blob }[]} */ members) {
    await ext.prepara(members.map(x => x.name));
    const rd = reader(members.map(x => x.blob), undefined);
    return native_samples(members.map(x => x.name), Float64Array.from(members.map(x => x.blob.size)), rd);
  }

  return { stubOf, attach, put, indexJob, toMzml, samples, sweep, srm: async (/** @type {string} */ name) => srmDi(await stubOf(name) || jerr("err.session.fileNotFound"), memberFiles, native_srm, ext), forget: (/** @type {string} */ n) => stubs.delete(n) };
}
