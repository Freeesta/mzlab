// @ts-strict
// The readers modules in the workers of the ENGINE (rust-worker.js, idx-worker.js): the engine (mzlab-wasm) has no reader for a Thermo `.raw`; the dataset is opened on the `thermo` module, loaded in the same
// worker on the first `.raw` (carica-modulo.js, never for other files), and the engine reads it through `ExternalReader` (crates/mzlab-wasm/src/external.rs): synchronous calls, no mzML in between.
// The index crosses as one Uint8Array and a scan as its head and views of its columns, read by the engine (`transport.rs`); the engine refuses a module whose `protocollo()` is not its own (err.file.wiffLoad).
// `native_external(open)` of the engine takes `open(names, sizes, read)` -> the object of a session of the module, or null for a dataset no module reads. The module is loaded BEFORE the engine asks
// (`prepara`, async): the engine cannot wait for a download.
import { loadModulo, trapGuard } from "./carica-modulo.js";

/** the family whose module reads a dataset, by the name of its members (the same rule as `LETTORI_FAM` in lettori-famiglie.js, for the families the engine reads through a module); "" when none does */
export function famigliaEsterna(/** @type {string[]} */ names) {
  if (names.length === 1 && /\.mzml$/i.test(names[0])) return "aperti";      // the engine refuses an mzML with numpress arrays; the page makes a dataset of one only for those
  return names.some(n => /\.raw$/i.test(n)) ? "thermo" : "";
}

/** the error the page translates when the module cannot be loaded (thrown as the engine's own errors: a JSON string) */
function loadError(/** @type {string} */ fam, /** @type {any} */ e) {
  return JSON.stringify({ error_key: "err.file.moduleLoad", params: { family: fam, detail: String(e && e.message || e) } });
}

/** the methods of a session, each through `g` */
const wrap = (/** @type {(f: Function) => Function} */ g, /** @type {Record<string, Function>} */ o) => Object.fromEntries(Object.entries(o).map(([k, f]) => [k, g(f)]));

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
    // the module reads the members with the engine's own `read`; the SHA-1 of a member the page knows is not read again
    // a call that traps forgets the module here and in carica-modulo.js: the next `prepara` loads a fresh instance
    const g = (/** @type {Function} */ f) => trapGuard(fam, f, () => loaded.delete(fam));
    const h = g(() => w.open_with_sha1 && sums.some(Boolean) ? w.open_with_sha1(names, sizes, read, sums) : w.open(names, sizes, read))();
    return wrap(g, {
      // the version of the binary transport of the index and the scans (a module of an older build has none)
      protocol: () => (w.protocol ? w.protocol() : 1),
      samples: () => w.samples(h),
      // one Uint8Array: the columns of the index (crates/mzlab-core/src/transport.rs)
      index: (/** @type {number} */ c) => w.index(h, c),
      scan: (/** @type {number} */ c, /** @type {number} */ k) => w.scan(h, c, k),
      // the scan as its head and views of the module's columns, copied by the engine in one piece each (a module of an older build has only `scan`)
      ...(w.scan_parts ? { scan_parts: (/** @type {number} */ c, /** @type {number} */ k) => w.scan_parts(h, c, k) } : {}),
      scan_header: (/** @type {number} */ c, /** @type {number} */ k) => w.scan_header(h, c, k),
      chromatograms: (/** @type {number} */ c) => w.chromatograms(h, c),
      chromatogram: (/** @type {number} */ c, /** @type {string} */ id) => w.chromatogram(h, c, id),
      method: (/** @type {number} */ c) => w.method(h, c),
      file_header: (/** @type {number} */ c) => w.file_header(h, c),
      // the spectra a truncated file declares but does not hold (a module of an older build has none)
      missing: (/** @type {number} */ c) => (w.missing ? w.missing(h, c) : 0),
      // why: `warn.file.truncated`, `warn.raw.eventsRefused`, `warn.raw.damaged` (empty for a module of an older build)
      cause: (/** @type {number} */ c) => (w.cause ? w.cause(h, c) : ""),
      close: () => w.close(h),
    });
  };
  return {
    /** loads the module of the family that reads the dataset with these members (nothing for the others) and registers `open` on the engine; rejects with the JSON `err.file.moduleLoad` */
    async prepara(/** @type {string[]} */ names, /** @type {{ name: string, size: number, sha1?: string }[] | undefined} */ members) {
      // a member without a SHA-1 must not inherit the one of an earlier member with the same name and size
      for (const m of members || []) { const k = m.name + "\0" + m.size; if (m.sha1) known.set(k, m.sha1); else known.delete(k); }
      const fam = famigliaEsterna(names);
      if (!fam) return;
      if (!registered) { native_external(open); registered = true; }
      if (loaded.has(fam)) return;
      try { loaded.set(fam, await loadModulo(fam)); } catch (e) { throw loadError(fam, e); }
    },
    /** a session of the module over the members of a dataset, for the page's own reads (the series of the instrument log); `prepara` first */
    open,
    loaded: () => [...loaded.keys()],
  };
}
