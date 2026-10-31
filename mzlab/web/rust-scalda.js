// @ts-strict
// Warm-up of the pointed folder, engine side (docs/agenti/riscaldamento.md): `warm` copies an mzML to OPFS and builds its index exactly as an upload does, then closes
// its slot and leaves a marker (OPFS folder "warm"); lr_boot hides the marked files. `claim` (the file is opened by the student) takes the marker away and opens the index,
// so the normal load skips the copy and the build. Loaded by rust-worker.js on the first warm/claim, never at start-up.
/** @typedef {{ size: number, mtime: number, bytes: number, at: number, missing: number }} Marker */
const SHARE = 0.25, SPARE = 256 << 20;           // the warm files take at most a quarter of the quota and always leave 256 MB free

/** @param {{ ARCH: any, lrPut: (m: any) => Promise<any>, lrIndex: (m: any) => Promise<any>, lrOpen: (n: string) => Promise<boolean>, lrFile: Map<string, File>, closeSlot: (n: string) => void }} d */
export function createScalda({ ARCH, lrPut, lrIndex, lrOpen, lrFile, closeSlot }) {
  const key = (/** @type {string} */ n) => n + ".json";
  /** @param {string} name @returns {Promise<Marker | null>} */
  async function marker(name) {
    const f = await ARCH.get("warm", key(name)); if (!f) return null;
    try { return JSON.parse(await f.text()); } catch (_) { return null; }
  }
  async function all() {
    /** @type {{ name: string, m: Marker }[]} */ const out = [];
    for (const e of await ARCH.list("warm")) { const name = e.name.replace(/\.json$/, ""), m = await marker(name); if (m) out.push({ name, m }); }
    return out.sort((a, b) => a.m.at - b.m.at);
  }
  async function drop(/** @type {string} */ name) {
    closeSlot(name); lrFile.delete(name);
    await ARCH.remove("warm", key(name)); await ARCH.remove("files", name);
    for (const ix of [name + ".mzidx", name + ".cen.mzidx", name + ".mzidx.part"]) await ARCH.remove("idx", ix);
  }
  const fresh = async (/** @type {string} */ name, /** @type {Marker | null} */ mk, /** @type {number} */ size, /** @type {number} */ mtime) => {
    if (!mk || mk.size !== size || mk.mtime !== mtime) return false;
    const f = await ARCH.get("files", name), ix = await ARCH.get("idx", name + ".mzidx");
    return !!f && !!ix && f.size === size;
  };
  // the oldest warm files go until `need` bytes are free (a real load always wins over a guess)
  async function makeRoom(/** @type {number} */ need) {
    const ws = await all(); if (!ws.length) return;
    const e = await ARCH.estimate(); if (!e.quota) return;
    let free = e.quota - e.usage;
    for (const w of ws) { if (free >= need) break; await drop(w.name); free += w.m.bytes; }
  }
  /** @param {{ name: string, blob: Blob & { lastModified?: number }, mtime?: number, id?: number }} m */
  async function warm(m) {
    const size = m.blob.size, mtime = +(m.mtime ?? m.blob.lastModified ?? 0), mk = await marker(m.name);
    if (await fresh(m.name, mk, size, mtime)) return { info: JSON.stringify({ skipped: "done" }) };
    if (!mk && (lrFile.has(m.name) || await ARCH.get("files", m.name))) return { info: JSON.stringify({ skipped: "loaded" }) };      // the student has it already
    const e = await ARCH.estimate(), used = (await all()).reduce((s, w) => s + w.m.bytes, 0), need = size * ARCH.ROOM;
    if (e.quota && (used + need > e.quota * SHARE || e.quota - e.usage - need < SPARE)) return { info: JSON.stringify({ skipped: "room" }) };
    try {
      const put = await lrPut({ ...m, nat: null }), ixr = JSON.parse((await lrIndex({ ...m, cen: false })).info || "{}");
      const ix = await ARCH.get("idx", m.name + ".mzidx");
      closeSlot(m.name); await ARCH.dropLease("idx", m.name + ".mzidx");
      /** @type {Marker} */ const out = { size, mtime, bytes: JSON.parse(put.info).size + (ix ? ix.size : 0), at: Date.now(), missing: ixr.missing || 0 };
      await ARCH.put("warm", key(m.name), new TextEncoder().encode(JSON.stringify(out)));
    } catch (_) {
      await drop(m.name);                                                     // silent: the student sees the normal error when the file is opened
      return { info: JSON.stringify({ skipped: "failed" }) };
    } finally { ARCH.release(m.name); }
    return { info: JSON.stringify({ warmed: true }) };
  }
  /** @param {{ name: string, blob: Blob & { lastModified?: number }, mtime?: number }} m */
  async function claim(m) {
    const size = m.blob.size, mtime = +(m.mtime ?? m.blob.lastModified ?? 0), mk = await marker(m.name);
    if (!mk) return { info: JSON.stringify({ hit: false }) };
    if (!(await fresh(m.name, mk, size, mtime)) || !(await lrOpen(m.name).catch(() => false))) { await drop(m.name); return { info: JSON.stringify({ hit: false }) }; }
    await ARCH.remove("warm", key(m.name));
    const f = await ARCH.get("files", m.name); if (f) lrFile.set(m.name, f);
    return { info: JSON.stringify({ hit: true, size, missing: mk.missing || 0 }), files: f ? [[m.name, f]] : [] };
  }
  return { lr_warm: (/** @type {any} */ m) => warm({ ...m, warm: true }), lr_warm_claim: claim, makeRoom, names: async () => new Set((await all()).map(w => w.name)), clear: async () => { for (const w of await all()) await drop(w.name); await ARCH.clear("warm"); } };
}
