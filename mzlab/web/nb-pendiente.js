// @ts-check
// The notebook write goes page -> engine worker -> IndexedDB, and a reload can kill it half way (the engine busy with a file): the last flush stays
// here, in localStorage, until the engine confirms it; the next page load writes it again if it is newer than the stored copy (explore.js nbSave / nbLoad).
const NBP = (() => {
  const KEY = "mzlab.nbpending";
  return {
    /** Keeps `body` (the JSON of the notebook just sent) until the engine says it has it. */
    keep(/** @type {string} */ body) { try { localStorage.setItem(KEY, body); } catch (_) { /* private mode: the engine write alone */ } },
    /** The engine has `body`: nothing to keep any more. */
    done(/** @type {string} */ body) { try { if (localStorage.getItem(KEY) === body) localStorage.removeItem(KEY); } catch (_) { /* nothing */ } },
    /** The unconfirmed flush of the previous page if it is newer (time stamp `_t`) than the stored notebook `stored`, else null; it is removed either way. */
    take(/** @type {Record<string, any>} */ stored, /** @type {(j: any) => Record<string, any>} */ clean) {
      try {
        const raw = localStorage.getItem(KEY); if (!raw) return null;
        localStorage.removeItem(KEY);
        const q = clean(JSON.parse(raw));
        return q._t && q._t > (stored._t || 0) ? q : null;
      } catch (_) { return null; }
    },
  };
})();
