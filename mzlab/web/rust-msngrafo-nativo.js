// @ts-strict
// What api/msn_grafo reads when the infusion is a sample of a maker's file (.raw ...): the .wasm msngrafo only decodes an mzML by offset, so the sample is
// written once as mzML on OPFS (conv/) with its own index (idx/), kept for the next request of the same sample; the page shows one bar over the phases.
// Used by rust-worker.js (op "msngrafo"). docs/agenti/lettura-nativa.md.

/** Share of the bar of each phase when the sample has to be converted first. */
const PESI = { convert: [0, 0.5], index: [0.5, 0.65], graph: [0.65, 1] };
const TICK = 250;

/** @typedef {{ phase: string, frac: number, done?: number, total?: number }} Avanza */
/** @typedef {(p: Avanza) => void} Annuncia */

/** The file name of the derived mzML: the sample and the members it stands for, so a replaced file never reads an old one. @param {string} name @param {any} st */
export function chiaveDerivata(name, st) {
  const sig = JSON.stringify([st.c, st.centroid, (st.members || []).map((/** @type {any} */ x) => [x.name, x.size, x.mtime])]);
  let h = 5381;
  for (let i = 0; i < sig.length; i++) h = ((h * 33) ^ sig.charCodeAt(i)) >>> 0;
  return "msn~" + name.replace(/[^\w.-]/g, "_") + "~" + h.toString(36);
}

/** The frac of the whole bar for `frac` of the phase `phase`; without conversion the graph takes all of it. @param {boolean} nativo */
export const fraz = (nativo, phase, frac) => {
  const [a, b] = nativo ? /** @type {Record<string, number[]>} */ (PESI)[phase] : [0, 1];
  return a + (b - a) * Math.max(0, Math.min(1, frac));
};

/**
 * @param {{ nativo: { stubOf: (n: string) => Promise<any>, toMzml: (m: any, post: (p: any) => void) => Promise<any> }, ARCH: any,
 *   indexPlain: (name: string, blob: Blob, onProgress: (p: any) => void) => Promise<File | null> }} d
 * @returns {(name: string, annuncia: Annuncia) => Promise<{ idx: File, mzml: Blob } | null>} null = a plain mzML: the file of the page is read as it is
 */
export function createDerivata({ nativo, ARCH, indexPlain }) {
  /** @param {string} dest */
  async function convFile(dest) {
    try { return await (await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("conv")).getFileHandle(dest)).getFile(); } catch (_) { return null; }
  }
  return async (name, annuncia) => {
    const st = await nativo.stubOf(name);
    if (!st) return null;
    const key = chiaveDerivata(name, st), dest = key + ".mzML";
    let mzml = await convFile(dest), idx = mzml && mzml.size ? await ARCH.get("idx", dest + ".mzidx") : null;
    if (mzml && mzml.size && idx) return { idx, mzml };
    for (const e of await ARCH.list("idx")) if (e.name.startsWith(key.replace(/~[^~]*$/, "~")) && e.name !== dest + ".mzidx") await ARCH.remove("idx", e.name);
    annuncia({ phase: "convert", frac: 0 });
    const r = await nativo.toMzml({ name, dest, compression: "standard" }, p => annuncia({ phase: "convert", frac: p.scansTotal ? p.scans / p.scansTotal : 0, done: p.scans, total: p.scansTotal || undefined }));
    mzml = r.file || (r.parts ? new Blob(r.parts) : null);
    if (!mzml || !mzml.size) throw JSON.stringify({ error_key: "err.msngrafo.noMzml", params: {} });
    annuncia({ phase: "index", frac: 0 });
    idx = await indexPlain(dest, mzml, p => annuncia({ phase: "index", frac: p.frac || 0, done: p.done, total: p.total }));
    return { idx, mzml };
  };
}

/**
 * `readerFor` that tells how far the graph has read of the mzML (bytes read over its size, at most one message per TICK ms): the bar rises while the module works.
 * @param {(blob: Blob) => (offset: number, length: number) => Uint8Array} readerFor @param {Blob} blob @param {(p: Avanza) => void} annuncia
 * @returns {(b: Blob) => (offset: number, length: number) => Uint8Array}
 */
export function conAvanzamento(readerFor, blob, annuncia) {
  let letti = 0, ultimo = 0;
  return b => {
    const rd = readerFor(b);
    if (b !== blob) return rd;
    return (offset, length) => {
      const out = rd(offset, length);
      letti += length;
      const t = performance.now();
      if (t - ultimo > TICK) { ultimo = t; annuncia({ phase: "graph", frac: letti / Math.max(1, blob.size), done: letti, total: blob.size }); }
      return out;
    };
  };
}
