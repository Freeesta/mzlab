// @ts-strict
// The operations of rust-worker.js on `metodo/1` (the method of an item, docs/agenti/method.md): the document the mzML carries, the one of the .wiff it was made from, or the .dam preselected for it.
// A .wiff dropped without its .scan is only read for its method: its SHA-1 is looked for among the source files the open mzML files name (ProteoWizard writes it), and the document of the matching
// sample is kept for those files until the page is closed.
import { loadModulo } from "./carica-modulo.js";

export function createMetodoOps({ engine, ARCH, readerFor, open, sync }) {
  const wiffDocs = new Map();               // mzML name -> metodo/1 JSON of the sample it was made from
  const base = n => String(n).split("#")[0];
  async function register(m) {
    // the document of a sample comes from the `sciex` readers module (the extended metadata, as the CLI writes it); the engine .wasm has no room for it
    let sciex = null;                       // loaded when a sample matches, not before
    const e = engine(), f = m.blob, rd = readerFor(f), sha = e.sha1_file(f.size, rd), found = [];
    for (const x of (await ARCH.list("files")).filter(x => /\.mzml$/i.test(x.name) && !/\.part$/.test(x.name))) {
      const g = await ARCH.get("files", x.name);
      if (!g) continue;
      const s = JSON.parse(e.mzml_sources(g.size, readerFor(g)));
      if (!s.sources.some(q => q.sha1 === sha) || !s.sample) continue;
      sciex ??= await loadModulo("sciex");
      wiffDocs.set(x.name, sciex.wiff_metodo(f.name, f.size, (/** @type {number} */ _i, /** @type {number} */ o, /** @type {number} */ n) => rd(o, n), s.sample, sha));
      found.push({ name: x.name, sample: s.sample });
    }
    return { info: JSON.stringify({ sha1: sha, matched: found }) };
  }
  return async function metodoOp(m) {
    switch (m.op) {
      case "lr_metodo_wiff": return register(m);
      // a *.m method folder: its files come in one buffer
      case "lr_metodo_m": return { info: engine().agilent_m_metodo(m.name, m.names, m.lens, m.data) };
      case "lr_metodo": {                    // { source: "mzml" | "wiff" | "dam" | "none", metodo: {...} }
        await sync();
        const o = await open(m), e = engine();
        // a sample of a maker's file: from its reader
        const emb = e.native_metodo(o.slot) || e.mzml_metodo(o.f.size, o.rd);
        let doc = emb || wiffDocs.get(base(m.name)) || e.lab_metodo(o.slot, m.name, o.f.size, o.rd);
        let source = emb ? "mzml" : wiffDocs.has(base(m.name)) ? "wiff" : doc ? "dam" : "none";
        // an mzML written by another program: its header (a part of a DDA file, «name#MS1», reads the header of its file)
        const file = String(m.name).split("#")[0];
        if (!doc && /\.mzml$/i.test(file)) { doc = e.mzml_head_metodo(file, o.f.size, o.rd); if (doc) source = "mzml"; }
        return { info: `{"source":"${source}","metodo":${doc || "null"}}` };
      }
    }
    return undefined;
  };
}
