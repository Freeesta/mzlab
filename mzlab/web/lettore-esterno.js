// @ts-check
// The readers modules in the workers of the ENGINE (rust-worker.js, idx-worker.js): the engine (mzlab-wasm) has no reader for a Thermo `.raw`; the dataset is opened on the `thermo` module, loaded in the same
// worker on the first `.raw` (carica-modulo.js, never for other files), and the engine reads it through `LettoreEsterno` (crates/mzlab-wasm/src/esterno.rs): synchronous calls, no mzML in between.
// The index and the scans cross as one Uint8Array each, read by the engine (`trasporto.rs`); the engine refuses a module whose `protocollo()` is not its own (err.file.wiffLoad).
// `native_external(open)` of the engine takes `open(names, sizes, read)` -> the object of a session of the module, or null for a dataset no module reads. The module is loaded BEFORE the engine asks
// (`prepara`, async): the engine cannot wait for a download.
import { loadModulo } from "./carica-modulo.js";

/** the family whose module reads a dataset, by the name of its members (the same rule as `LETTORI_FAM` in lettori-famiglie.js, for the families the engine reads through a module); "" when none does */
export function famigliaEsterna(/** @type {string[]} */ names) {
  return names.some(n => /\.raw$/i.test(n)) ? "thermo" : "";
}

/** the error the page translates when the module cannot be loaded (thrown as the engine's own errors: a JSON string) */
function loadError(/** @type {string} */ fam, /** @type {any} */ e) {
  return JSON.stringify({ error_key: "err.file.wiffLoad", params: { family: fam, detail: String(e && e.message || e) } });
}

/** @param {Function} native_external the one of the .wasm of this worker */
export function createEsterno(native_external) {
  /** @type {Map<string, any>} the modules loaded, by family */
  const loaded = new Map();
  let registered = false;
  /** @type {Map<string, string>} the SHA-1 of the members the page took while it copied them, by name and size (`prepara`) */
  const known = new Map();
  const open = (/** @type {string[]} */ names, /** @type {Float64Array} */ sizes, /** @type {Function} */ read, /** @type {string[] | undefined} */ sha1) => {
    const fam = famigliaEsterna(Array.from(names));
    if (!fam) return null;
    const w = loaded.get(fam);
    if (!w) throw loadError(fam, "module not loaded");
    const sums = Array.from(names, (n, i) => (sha1 && sha1[i]) || known.get(n + "\0" + sizes[i]) || "");
    const h = w.apri_con_sha1 && sums.some(Boolean) ? w.apri_con_sha1(names, sizes, read, sums) : w.apri(names, sizes, read);        // the module reads the members with the engine's own `read`; the SHA-1 of a member the page knows is not read again
    return {
      protocollo: () => (w.protocollo ? w.protocollo() : 1),        // the version of the binary transport of the index and the scans (a module of an older build has none)
      campioni: () => w.campioni(h),
      indice: (/** @type {number} */ c) => w.indice(h, c),        // one Uint8Array: the columns of the index (crates/mzlab-core/src/trasporto.rs)
      scansione: (/** @type {number} */ c, /** @type {number} */ k) => w.scansione(h, c, k),
      intestazione: (/** @type {number} */ c, /** @type {number} */ k) => w.intestazione(h, c, k),
      cromatogrammi: (/** @type {number} */ c) => w.cromatogrammi(h, c),
      cromatogramma: (/** @type {number} */ c, /** @type {string} */ id) => w.cromatogramma(h, c, id),
      metodo: (/** @type {number} */ c) => w.metodo(h, c),
      testata: (/** @type {number} */ c) => w.testata(h, c),
      chiudi: () => w.chiudi(h),
    };
  };
  return {
    /** loads the module of the family that reads the dataset with these members (nothing for the others) and registers `open` on the engine; rejects with the JSON `err.file.wiffLoad` */
    async prepara(/** @type {string[]} */ names, /** @type {{ name: string, size: number, sha1?: string }[] | undefined} */ members) {
      for (const m of members || []) if (m.sha1) known.set(m.name + "\0" + m.size, m.sha1);
      const fam = famigliaEsterna(names);
      if (!fam) return;
      if (!registered) { native_external(open); registered = true; }
      if (loaded.has(fam)) return;
      try { loaded.set(fam, await loadModulo(fam)); } catch (e) { throw loadError(fam, e); }
    },
    /** a session of the module over the members of a dataset, for the page's own reads (the series of the instrument log); `prepara` first */
    apri: open,
    loaded: () => [...loaded.keys()],
  };
}
