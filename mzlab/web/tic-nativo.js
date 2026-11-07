// @ts-strict
// The provisional TIC of a Thermo .raw while it loads (rust-nativo.js): a free builder of the index pool (idx-worker.js, op "early") opens the dropped files on the `thermo`
// module and reads the MS1 rt/tic of the scan index (`native_early_tic` of the engine) while the engine's worker copies the members to OPFS. The TIC reaches the page as
// `early_tic` of the progress of the rpc that loads the sample at that moment (the copy, then the index), as the scan table of an mzML does (idx-aiuti.js earlyTicInputs).

/** @param {() => { build: (job: any) => Promise<any> }} pool @param {(m: any) => void} post */
export function createTicNativo(pool, post) {
  /** @type {Map<string, { id: number, p: any, tic: any }>} the samples being loaded: the rpc now loading each, its last progress, the TIC once read */
  const by = new Map();
  return {
    /** starts the read for the lr_put `m` of a sample of a .raw (nothing for the other makers: their open reads more than the scan index) */
    start(/** @type {any} */ m) {
      const nat = m.nat;
      if (!nat || !nat.members.some((/** @type {{ name: string }} */ x) => /\.raw$/i.test(x.name))) return;
      const e = { id: m.id, p: { frac: 0, phase: "read" }, tic: null };
      by.set(m.name, e);
      pool().build({ name: m.name, msg: { op: "early", names: nat.members.map((/** @type {{ name: string }} */ x) => x.name), files: nat.members.map((/** @type {{ blob: Blob }} */ x) => x.blob), c: nat.c } }).then(r => {
        if (by.get(m.name) !== e || !r.ok || !r.stats || r.stats.rt.length < 2) return;      // loaded again, cancelled, or not read: no provisional TIC, the real one comes with the index
        e.tic = r.stats;
        post({ ...e.p, op: "progress", id: e.id, early_tic: e.tic });
      });
    },
    /** the progress `p` of the rpc `id` that loads `name` now: a TIC read later goes with it */
    tell(/** @type {string} */ name, /** @type {number} */ id, /** @type {any} */ p) { const e = by.get(name); if (e) { e.id = id; e.p = p; } },
    drop: (/** @type {string} */ name) => { by.delete(name); },
  };
}
