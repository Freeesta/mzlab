// Where the mzML of a converted .raw goes (rust-worker.js): a file of OPFS written piece by piece, so the mzML (GBs for a profile run) is never held in memory.
// `dest` = file name in mzlab/conv/ chosen by the page (it removes it if the conversion is cancelled). Without OPFS (or a sync handle) the pieces are kept in memory instead.
// Old files of earlier visits are removed here (a day old).
import { writeFull, quotaError } from "./archivio.js";
export async function rawSink(dest) {
  let dir = null, fh = null, ah = null;
  try {
    dir = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("conv", { create: true });
    for await (const [n, h] of dir.entries()) { if (h.kind === "file" && n !== dest) { const f = await h.getFile(); if (Date.now() - f.lastModified > 864e5) await dir.removeEntry(n).catch(() => {}); } }
    fh = await dir.getFileHandle(dest, { create: true });
    ah = await fh.createSyncAccessHandle();
    ah.truncate(0);
  } catch (_) { if (ah) ah.close(); ah = null; }
  if (!ah) {
    const parts = [];
    return { write: p => parts.push(p), abort: async () => {}, error: () => null, finish: async () => ({ parts }) };
  }
  let at = 0, full = false;
  return {
    // a full disk makes write() return less than asked (or throw): the mzML would be cut short, so the conversion stops with `error` (error.quota) instead of ending "well"
    write: p => { if (full) throw new Error("storage full"); const n = writeFull(ah, p, at); at += n; if (n < p.length) { full = true; throw new Error("storage full"); } },
    abort: async () => { try { ah.close(); } catch (_) { /* closed */ } await dir.removeEntry(dest).catch(() => {}); },
    // the error the page shows if the disk filled up, else null; the mzML must also END as an mzML (a cut-short file is the cause of "cannot be read" later)
    error: () => full ? quotaError(at) : null,
    finish: async () => {
      ah.flush(); ah.close();
      const file = await fh.getFile();
      if (file.size !== at) throw JSON.stringify({ error_key: "err.file.raw", params: { detail: "incomplete" } });
      return { file };
    },
  };
}
