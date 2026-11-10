// @ts-strict
// Where the mzML of a converted .raw goes (rust-worker.js): a file of OPFS written piece by piece, so the mzML (GBs for a profile run) is never held in memory.
// `dest` = file name in mzlab/conv/ chosen by the page (it removes it if the conversion is cancelled). Without OPFS (or a sync handle) the pieces are kept in memory instead,
// up to `capMb` (the engine's budget; 0 = no cap): past it the conversion stops with the calm error.memoryBudget (phase mzmlInMemory) instead of the tab being killed (Safari).
// Old files of earlier visits are removed here (a day old).
import { writeFull, quotaError } from "./archivio.js";
const MB = 1048576;
/** error.memoryBudget as `budget::refusal` of Rust writes it: `held` = bytes the mzML had reached when it was stopped */
const inMemoryRefusal = (/** @type {number} */ held, /** @type {number} */ capMb) => JSON.stringify({ error_key: "error.memoryBudget",
  params: { fase: "mzmlInMemory", stima_mb: Math.ceil(held / MB), budget_mb: Math.floor(capMb), libero_mb: Math.floor(capMb), eccesso_mb: Math.max(1, Math.ceil((held - capMb * MB) / MB)) } });
export async function rawSink(/** @type {string} */ dest, capMb = 0) {
  /** @type {any} */ let dir = null, fh = null, ah = null;
  try {
    dir = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("conv", { create: true });
    for await (const [n, h] of dir.entries()) { if (h.kind === "file" && n !== dest) { const f = await h.getFile(); if (Date.now() - f.lastModified > 864e5) await dir.removeEntry(n).catch(() => {}); } }
    fh = await dir.getFileHandle(dest, { create: true });
    ah = await fh.createSyncAccessHandle();
    ah.truncate(0);
  } catch (_) { if (ah) ah.close(); ah = null; }
  if (!ah) {
    /** @type {Uint8Array[]} */ let parts = [];
    let held = 0, over = false;
    return {
      write: (/** @type {Uint8Array} */ p) => {
        if (over) throw new Error("memory budget");
        held += p.length;
        if (capMb > 0 && held > capMb * MB) { over = true; parts = []; throw new Error("memory budget"); }   // the pieces go at once: the page must stay alive to show the notice
        parts.push(p);
      },
      abort: async () => { parts = []; }, error: () => over ? inMemoryRefusal(held, capMb) : null, finish: async () => ({ parts }) };
  }
  let at = 0, need = 0, full = false;      // `at` = bytes really written, `need` = bytes the file needs once the failing block is counted
  return {
    // a full disk makes write() return less than asked (or throw): the mzML would be cut short, so the conversion stops with `error` (error.quota) instead of ending "well"
    write: p => { if (full) throw new Error("storage full"); const n = writeFull(ah, p, at); at += n; need = at; if (n < p.length) { need = at + (p.length - n); full = true; throw new Error("storage full"); } },
    abort: async () => { try { ah.close(); } catch (_) { /* closed */ } await dir.removeEntry(dest).catch(() => {}); },
    // the error the page shows if the disk filled up, else null; the mzML must also END as an mzML (a cut-short file is the cause of "cannot be read" later)
    error: () => full ? quotaError(need, at) : null,
    finish: async () => {
      ah.flush(); ah.close();
      const file = await fh.getFile();
      if (file.size !== at) throw JSON.stringify({ error_key: "err.file.raw", params: { detail: "incomplete" } });
      return { file };
    },
  };
}
