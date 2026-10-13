// @ts-check
// TPMINE-PRIVATE  Ingress of mzFinder: every mzML the page sends to the worker gets its .mzidx in OPFS (mzlab/idx/<name>.mzidx) before the Rust engine reads it.
// The index is built by the SAME code as the main program (prepare_upload of mzlab_wasm.js for the scan table, the pool of idx-worker.js for the index, ARCH.promote for the
// final name), so the bytes are the ones the Data tab would have written. A file that already has a fresh index (same size recorded in its meta) is left alone.
// Imported by tpmine-worker.js from <site>/mzfinder/finder-idx.js; `base` is the URL of the site's static folder (the one with wasm/, idxpool.js, idx-worker.js, archivio.js).

/** @param {Blob} blob */
const readerFor = blob => {
  const fr = new FileReaderSync();
  return (/** @type {number} */ off, /** @type {number} */ len) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(off, off + len)));
};

/** the size of the source file that a .mzidx records in its meta (header: "MZIDX001", u32, u32, u64 meta_off, u64 meta_len), null when it cannot be read @param {File} ix */
async function recordedSize(ix) {
  try {
    const head = new DataView(await ix.slice(0, 32).arrayBuffer());
    const off = Number(head.getBigUint64(16, true)), len = Number(head.getBigUint64(24, true));
    return JSON.parse(await ix.slice(off, off + len).text()).source.size;
  } catch (_) { return null; }
}

/** @param {string} name */
async function idxFile(name) {
  try {
    const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx");
    return await (await d.getFileHandle(name + ".mzidx")).getFile();
  } catch (_) { return null; }
}

/** @param {string} base @param {(text: string) => void} say */
export function createIngress(base, say) {
  /** @type {Promise<any> | null} */ let lib = null;
  /** the index builder: the compiled .wasm of the main program, its scan-table function, the pool of workers and the archive helpers (made at the first file that needs an index) */
  const tools = () => lib || (lib = (async () => {
    const url = base + "wasm/mzlab_wasm_bg.wasm";
    const module = await WebAssembly.compileStreaming(fetch(url));
    const [wasm, pools, arch] = await Promise.all([import(base + "wasm/mzlab_wasm.js"), import(base + "idxpool.js"), import(base + "archivio.js")]);
    await wasm.default({ module_or_path: module });
    const pool = pools.createPool({ workerUrl: new URL(base + "idx-worker.js"), moduleUrl: url, module, size: pools.poolSize(false, navigator.hardwareConcurrency), budget: 64 });
    return { wasm, pool, arch };
  })().catch(e => { lib = null; throw e; }));

  /** the builds under way, by file name @type {Map<string, Promise<void>>} */ const building = new Map();

  /** @param {string} name @param {Blob} blob */
  async function build(name, blob) {
    const { wasm, pool, arch } = await tools(), part = name + ".mzidx.mzf.part";
    const inputs = Array.from(wasm.prepare_upload(blob.size, readerFor(blob), name)).slice(0, 8);
    const res = await pool.build({ name, blob, inputs, part, onProgress: (/** @type {{frac: number}} */ p) => say(`Indexing ${name}... ${Math.round(100 * p.frac)}%`) });
    if (!res.ok) {
      try { await arch.remove("idx", part); } catch (_) { /* nothing written */ }
      let why = String(res.text || res.reason); try { why = JSON.parse(why).error_key || why; } catch (_) { /* plain text */ }
      throw new Error(`cannot index ${name}: ${why}`.slice(0, 300));
    }
    try { await arch.promote("idx", part, name + ".mzidx"); } catch (e) {
      // the main program indexes the same file at the same time (files restored from IndexedDB) and holds the final name: its index, with the same bytes, is the one to use
      try { await arch.remove("idx", part); } catch (_) { /* already gone */ }
      for (let i = 0; i < 40; i++) {
        const ix = await idxFile(name);
        if (ix && await recordedSize(ix) === blob.size) return;
        await new Promise(r => setTimeout(r, 250));
      }
      throw e;
    }
  }

  return {
    /** every named file the page sent (name -> Blob | ArrayBuffer) has a fresh .mzidx when this returns; one that cannot be indexed rejects @param {string[]} names @param {Map<string, any>} held */
    async ensure(names, held) {
      /** @type {Promise<void>[]} */ const jobs = [];
      for (const name of names) {
        const src = held.get(name);
        if (!src) continue;                              // not sent by the page: it keeps whatever index it has
        const blob = src instanceof Blob ? src : new Blob([src]), ix = await idxFile(name);
        if (ix && await recordedSize(ix) === blob.size) continue;
        say(`Indexing ${name}...`);
        let job = building.get(name);                    // two classify calls at once (Reload from Data and a restored list) must not write the same .part twice
        if (!job) { job = build(name, blob).finally(() => building.delete(name)); building.set(name, job); }
        jobs.push(job);                                  // the scan table is read here, the index is built by the pool while the next file is prepared
      }
      await Promise.all(jobs);
    },
  };
}
