// @ts-check
// A .mzML.gz or .mzXML.gz picked on the site (docs/agenti/lettori-aperti.md, «gzip»): the worker of the engine inflates it piece by piece into OPFS (`rust-carica.js` putGz /
// gunzipTo), never whole in memory. An mzML becomes a plain upload (a stand-in File named without .gz); an mzXML becomes a temporary .mzXML of conv/ that the readers module reads.
// Without the stream (no sync handles of OPFS, `?gz=memoria`) a .mzML.gz takes the older path, inflated in memory by the module `aperti`, with a notice. Loaded by shell.js.

const W = /** @type {any} */ (window);
const FORCE_MEMORY = /[?&]gz=memoria\b/.test(location.search);
/** the page's own id in the names of its temporary files: a later start removes the files of pages whose lock is free (rust-worker.js gzOrphans) */
const SID = Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
let locked = false;
const holdSession = () => {
  if (locked || !navigator.locks) return;
  locked = true;
  navigator.locks.request("mzlab-gzses:" + SID, () => new Promise(() => {})).catch(() => { locked = false; });
};
/** sizes from which the in-memory path warns (the module holds the whole inflated file) */
const WARN_BYTES = 50 << 20;

/** the first bytes of the inflated file as text, `n` at most, read through DecompressionStream and stopped there ("" when the browser has no DecompressionStream) */
async function head(/** @type {Blob} */ f, /** @type {number} */ n) {
  if (typeof DecompressionStream !== "function") return "";
  const rd = f.stream().pipeThrough(new DecompressionStream("gzip")).getReader(), parts = [];
  let got = 0;
  try {
    while (got < n) { const { done, value } = await rd.read(); if (done) break; parts.push(value); got += value.length; }
  } catch (_) { /* a cut or broken stream: what it gave */ }
  rd.cancel().catch(() => {});
  return new TextDecoder().decode(new Uint8Array(await new Blob(parts).slice(0, n).arrayBuffer()));
}

const engineError = (/** @type {any} */ r) => {
  let j = null; try { j = JSON.parse(r.error); } catch (_) { /* a text */ }
  return Object.assign(new Error(j ? j.error_key : String(r.error || "engine lost")), j ? { json: { error: j.error_key, ...j } } : {});
};

export const GZFLUSSO = {
  isGz: (/** @type {string} */ name) => /\.(mzml|mzxml)\.gz$/i.test(name),
  /** the worker of the engine can stream the gzip into OPFS: the site's engine (not the app, which reads the .gz natively) with sync handles */
  stream() {
    if (FORCE_MEMORY || W.MZLAB_DESKTOP || !W.QQQ_BROWSER) return false;
    const C = W.CAPACITA;
    return !C || C.ha("opfsSync");
  },
  /** true when the inflated head of a .mzML.gz has MS-Numpress arrays: that file keeps the module's path (it rewrites them) */
  async numpress(/** @type {File} */ f) {
    const M = W.LETTORI_MOD;
    if (!M) return false;
    // 1 MB of the inflated head is enough unless the first spectrum is longer (then up to the 16 MB `variante` reads)
    let t = await head(f, 1 << 20);
    if (t.includes("<mzML") && !t.includes("</spectrum>") && t.length >= 1 << 20) t = await head(f, 16 << 20);
    return (await M.variante(new File([t], f.name.replace(/\.gz$/i, "")))) === "aperti";
  },
  /** the File the upload sends for a .mzML.gz: named as its mzML, its bytes are the gzip; lr.js passes `gz` to the worker */
  stand(/** @type {File} */ f) {
    const s = new File([f], f.name.replace(/\.gz$/i, ""), { lastModified: f.lastModified });
    Object.defineProperty(s, "mzlabGz", { value: { name: f.name, size: f.size } });
    return s;
  },
  /** the .mzXML of a .mzXML.gz, inflated into conv/ by the worker (a File of OPFS, `mzlabTemp` = its name there); `onp(frac)` follows the bytes read; `signal` cancels */
  async mzxml(/** @type {File} */ f, /** @type {(frac: number) => void} */ onp, /** @type {AbortSignal | undefined} */ signal) {
    const rb = W.MZLAB_RUST_READY ? await W.MZLAB_RUST_READY : null;
    if (!rb || rb.disabled) throw Object.assign(new Error("err.file.gzStream"), { json: { error: "err.file.gzStream", error_key: "err.file.gzStream", params: {} } });
    holdSession();
    const name = f.name.replace(/\.gz$/i, ""), dest = `gz~${SID}~${Math.random().toString(36).slice(2, 8)}.mzXML`;
    const stop = () => { rb.rpc({ op: "lr_cancel", names: [f.name] }); };
    if (signal) signal.addEventListener("abort", stop, { once: true });
    try {
      const r = await rb.rpc({ op: "lr_gunzip", name: f.name, blob: f, dest }, (/** @type {{ frac: number }} */ p) => onp(p.frac));
      if (signal && signal.aborted) throw Object.assign(new Error("cancelled"), { cancelled: true });
      if (r.dead || r.error) throw engineError(r);
      const out = new File([r.files[0][1]], name);
      Object.defineProperty(out, "mzlabTemp", { value: dest });
      return out;
    } finally { if (signal) signal.removeEventListener("abort", stop); }
  },
  /** the temporary .mzXML is no longer needed (its mzML is written) */
  async drop(/** @type {string} */ dest) {
    try { await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("conv")).removeEntry(dest); } catch (_) { /* gone, or a later start removes it */ }
  },
  /** the notice of the in-memory path for a big .mzML.gz (the whole inflated file must fit in the memory of the module), else "" */
  memoryNote(/** @type {File} */ f) {
    return f.size >= WARN_BYTES ? W.I18N.t("load.note.gzMemory", { name: f.name, mb: Math.round(f.size / 1048576) }) : "";
  },
};
/** @type {any} */ (window).GZFLUSSO = GZFLUSSO;
export default GZFLUSSO;
