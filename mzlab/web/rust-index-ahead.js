// @ts-check
// @ts-strict
// Index ahead (site only): a sample ticked in the «Choose samples» dialog gets its `.mzidx` built while the dialog is open, right after its copy (rust-prefetch.js), under a name that
// does not depend on the final selection (`nat~<key>~<id>~pre`; the stub name does, through `wiffName`/`sample_labels`). The index bytes do not depend on that name either
// (the source of the index is named after the members, `upload::set_name`), so «Open selected» only MOVES the file to the final name (`adopt`, ARCH.promote: no copy, no second read or index).
// Untick and X cancel the build and remove the file. The app and the CLI read in place and have nothing to build ahead.

/** the identity of an index: what makes two stubs the same sample of the same stores (the name of the list entry is not part of it) @param {any} st */
export const sigOf = st => JSON.stringify([st.ds, st.c, st.id, !!st.centroid, st.members.map((/** @type {any} */ m) => [m.name, m.store, m.size, m.mtime || 0])]);

/**
 * @param {{ ARCH: any, pool: () => any, conf: () => { cap: number, budget: number | undefined, isolated: boolean }, taken: Map<string, string[]>, lateJob: (job: any, isolated: boolean) => Promise<any>, storeOf: (nat: any, x: any) => string, prefix: string, magic: string }} d
 */
export function createIndexAhead({ ARCH, pool, conf, taken, lateJob, storeOf, prefix, magic }) {
  /** @typedef {{ name: string, sig: string, cut: boolean, job: Promise<{ missing: number, cause: string } | null> }} Ahead */
  const rows = /** @type {Map<string, Ahead>} */ (new Map());         // `key#id` -> the index being built or built
  const stats = { built: 0, adopted: 0, missed: 0, atOpen: 0 };                    // read by the test hook `natstats`
  const files = (/** @type {string} */ n) => [n + ".mzidx", n + ".mzidx.part", n + ".mzidx.sp.part"];
  const wipe = async (/** @type {string} */ n) => { for (const f of files(n)) await ARCH.remove("idx", f).catch(() => {}); };

  /** builds the index of sample `id` (position `c`) whose stores are in place; `tell(f)` 0..1; resolves true when the file is there, false when it was cut, refused or failed (Open then builds as it always did) */
  async function build(/** @type {any} */ nat, /** @type {number} */ c, /** @type {number} */ id, /** @type {(f: number) => void} */ tell) {
    const tag = nat.key + "#" + id, name = prefix + nat.key + "~" + id + "~pre";
    const members = nat.members.map((/** @type {any} */ x) => { const store = storeOf(nat, x), o = taken.get(store); return { name: x.name, store, size: x.blob.size, mtime: x.blob.lastModified || 0, ...(o ? { sha256: o[0], sha1: o[1], probe: o[2] } : {}) }; });
    const got = await Promise.all(members.map((/** @type {any} */ m) => ARCH.get("files", m.store)));
    if (got.some((/** @type {any} */ f, /** @type {number} */ i) => !f || f.size !== members[i].size)) return false;
    const st = { mzlab: magic, ds: nat.key, c, id, centroid: false, members, size: members.reduce((/** @type {number} */ a, /** @type {any} */ m) => a + m.size, 0), dim: null };
    const row = /** @type {Ahead} */ ({ name, sig: sigOf(st), cut: false, job: Promise.resolve(null) });
    rows.set(tag, row);
    row.job = (async () => {
      const k = conf(), part = name + ".mzidx.part";
      await wipe(name);
      const job = await lateJob({ st, files: got }, k.isolated);
      if (row.cut) return null;
      const res = await pool().build({ name, nat: job, part, cap: k.cap, budget: k.budget, onProgress: (/** @type {any} */ p) => tell(Math.min(1, p.frac || 0)) });
      if (!res.ok || row.cut) { await wipe(name); return null; }
      await ARCH.promote("idx", part, name + ".mzidx");
      stats.built++;
      return { missing: (res.stats && res.stats.missing) || 0, cause: (res.stats && res.stats.cause) || "" };
    })().catch(async () => { await wipe(name); return null; });
    return !!(await row.job);
  }

  /** stops the build of sample `tag` (`key#id`) and removes its file; resolves when it is gone */
  async function drop(/** @type {string} */ tag) {
    const row = rows.get(tag);
    if (!row) return;
    row.cut = true; rows.delete(tag); pool().cancel(row.name);
    await row.job; await wipe(row.name);
  }

  return {
    build, drop, stats,
    /** the tags (`key#id`) being built or built, of dataset `key` */
    tags: (/** @type {string} */ key) => [...rows.keys()].filter(t => t.startsWith(key + "#")),
    /** stops (without waiting) the builds of `tags`: an unticked sample must not wait for a long build */
    cancel: (/** @type {string[]} */ tags) => { for (const t of tags) { const r = rows.get(t); if (r) { r.cut = true; pool().cancel(r.name); } } },
    /** the stub `st` of the final name is written: when its index was built ahead (waiting for one still running) it moves to `name` and its statistics come back, else null (the index is built now) */
    async adopt(/** @type {string} */ name, /** @type {any} */ st) {
      const tag = st.ds + "#" + st.id, row = rows.get(tag);
      if (!row) { stats.missed++; return null; }
      if (row.sig !== sigOf(st)) { stats.missed++; await drop(tag); return null; }
      const o = await row.job;
      rows.delete(tag);
      if (!o || !(await ARCH.get("idx", row.name + ".mzidx"))) { stats.missed++; await wipe(row.name); return null; }
      await ARCH.promote("idx", row.name + ".mzidx", name + ".mzidx");
      stats.adopted++;
      return o;
    },
  };
}
