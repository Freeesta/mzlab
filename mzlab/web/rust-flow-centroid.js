// @ts-check
// The ops of the worker behind `rb.flowCentroid` (rust-centroidi.js): the Flow's centroid copy of a profile mzML is kept as the index `<name>.cen.mzidx`, where the peaks of a profile file are found. It is reused only
// when its key (sha256 of the source + parameters + engine version, in `<name>.cen.key`) matches; the user's own `<name>.centroidi.mzML` of Settings counts when the same engine wrote it (its header says so).
/** @param {any} h the helpers of rust-worker.js */
export function createFlowCentroid(h) {
  const { ARCH, infoDi, nuovo, safe, version, lrPut, lrIndex, close, release, lrFile } = h;
  const hexOf = async (/** @type {string} */ s) => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s)))].map(b => b.toString(16).padStart(2, "0")).join("");
  async function key(/** @type {string} */ name, /** @type {string} */ params) {
    const ix = await ARCH.get("idx", name + ".mzidx"), info = ix && await safe(() => infoDi("idx", ix));
    return info && info.sorgente_sha256 ? hexOf(`${info.sorgente_sha256}|${params}|centroidi ${version()}`) : "";
  }
  return {
    /** → { have: "flow" | "settings" | "", key }: a copy made already for these parameters and this engine */
    async have(/** @type {any} */ m) {
      const k = await key(m.name, m.params), mark = await ARCH.get("idx", m.name + ".cen.key");
      if (k && mark && await ARCH.get("idx", m.name + ".cen.mzidx") && (await mark.text()) === k) return { have: "flow", key: k };
      const copy = m.name.replace(/\.mzML$/i, ".centroidi.mzML"), f = !m.params && k ? await ARCH.get("files", copy) : null, cx = f && await ARCH.get("idx", copy + ".mzidx");
      if (cx && !nuovo("idx", await safe(() => infoDi("idx", cx))) && (await f.slice(0, 32768).text()).includes(`id="mzlab-centroidi" version="${version()}"`)) {
        await ARCH.put("idx", m.name + ".cen.mzidx", cx); await ARCH.put("idx", m.name + ".cen.key", k);
        return { have: "settings", key: k };
      }
      return { have: "", key: k };
    },
    /** the copy made by convertCentroids -> its index under the name of the original; the copy itself is not kept */
    async adopt(/** @type {any} */ m) {
      const tmp = m.name + ".cen-tmp.mzML";
      try {
        await lrPut({ name: tmp, blob: m.blob }); await lrIndex({ name: tmp, id: m.id });
        await ARCH.promote("idx", tmp + ".mzidx", m.name + ".cen.mzidx");
        await ARCH.put("idx", m.name + ".cen.key", await key(m.name, m.params));
      } finally {
        close(tmp); release(tmp);
        await ARCH.remove("files", tmp); await ARCH.remove("idx", tmp + ".mzidx"); lrFile.delete(tmp);
      }
    },
  };
}
