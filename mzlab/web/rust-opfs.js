// Where the mzML of a converted .raw goes (rust-worker.js): a file of OPFS written piece by piece, so the mzML (GBs for a profile run) is never held in memory.
// `dest` = file name in mzlab/conv/ chosen by the page (it removes it if the conversion is cancelled). Without OPFS (or a sync handle) the pieces are kept in memory instead.
// Old files of earlier visits are removed here (a day old).
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
    return { write: p => parts.push(p), abort: async () => {}, finish: async () => ({ parts }) };
  }
  let at = 0;
  return {
    write: p => { at += ah.write(p, { at }); },
    abort: async () => { ah.close(); await dir.removeEntry(dest).catch(() => {}); },
    finish: async () => { ah.flush(); ah.close(); return { file: await fh.getFile() }; },
  };
}
