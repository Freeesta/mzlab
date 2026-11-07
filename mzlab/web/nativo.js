// @ts-strict
"use strict";
// Files of a maker (Thermo .raw, SCIEX .wiff + .wiff.scan, Agilent .D folder) are loaded as they are (rust-nativo.js): no mzML is made. A sample is listed as `x.mzML` (a File with the members attached:
// `mzlabNat`, read by lr.js) and goes through upload() like any mzML; «Download .mzML» writes its mzML on request. Classic script, loaded after conversione.js; the pure helpers are run by node in tests/test_nativo.py.

const NATIVO = {
  // one key for the dataset (its samples share the members in the browser storage): the name without odd characters + a random part
  key(ds) { return String(ds).replace(/[^\w.-]+/g, "_").slice(0, 60) + "-" + Math.random().toString(36).slice(2, 8); },
  // the File that stands for sample `c` (number `id`) of a dataset in the list of the files; `members` = [{ name, blob }], names relative to the dataset
  sample(name, key, members, c, id, centroid, sampleSize) {
    const f = new File([JSON.stringify({ mzlab: "nat1", ds: key })], name),
      size = sampleSize != null ? sampleSize : (members.length === 1 ? members[0].blob.size : null);
    Object.defineProperty(f, "mzlabNat", { value: { key, members, c, id, centroid: !!centroid, size } });
    return f;
  },
  // the size to show: the one of the sample or members
  size(f) { return f && f.mzlabNat ? f.mzlabNat.size : f && f.mzlabModulo ? f.mzlabModulo.size : f.size; },
  // the engine of the page when it reads the files of a maker as they are, else null
  async engine() { const rb = window.MZLAB_RUST_READY ? await window.MZLAB_RUST_READY : null; return rb && !rb.disabled && rb.nativo ? rb : null; },
  // Thermo .raw files: each is one sample; the other files go on as they are
  raws(files, taken) {
    const out = [];
    for (const f of files) {
      if (!/\.raw$/i.test(f.name)) { out.push(f); continue; }
      out.push(NATIVO.sample(convName(f.name, [...taken, ...files.filter(x => !/\.raw$/i.test(x.name)).map(x => x.name), ...out.map(x => x.name)]), NATIVO.key(f.name), [{ name: f.name, blob: f }], 0, 1, false));
    }
    return out;
  },
  // mzML files with MS-Numpress arrays (`isNumpress(f)`: the family `aperti` of the readers modules): each is one sample read as it is through the module, no mzML is written; the other files go on as they are
  async numpress(files, isNumpress) {
    const out = [];
    for (const f of files) {
      out.push(/\.mzml$/i.test(f.name) && !f.mzlabNat && !f.mzlabModulo && await isNumpress(f) ? NATIVO.sample(f.name, NATIVO.key(f.name), [{ name: f.name, blob: f }], 0, 1, false) : f);
    }
    return out;
  },
  // the desktop app: its Files are stubs without bytes (desktop-shim.js) that no module of the page can read, and its native side reads every family. A file with an extension only a module
  // reads (`exts`) or an mzML the native readers take (`maybe(f)`, then MS-Numpress: a plain one is refused) becomes the samples the native side lists (`samplesOf`), named as the module names its mzML (`nameOf`)
  async desktop(files, exts, maybe, samplesOf, nameOf) {
    const out = [];
    for (const f of files) {
      const n = f.name.toLowerCase(), only = exts.some(e => n.endsWith(e));
      if (f.mzlabNat || f.mzlabModulo || !(only || n.endsWith(".mzml") && await maybe(f))) { out.push(f); continue; }
      const mem = [{ name: f.name, blob: f }], s = await samplesOf(mem).then(r => r.samples, () => null);
      if (!s || !s.length) { out.push(f); continue; }
      const key = NATIVO.key(f.name);
      s.forEach((x, c) => out.push(NATIVO.sample(!only ? f.name : nameOf(s.length > 1 ? x.name : f.name), key, mem, c, x.id, false, s.length > 1 ? x.size : undefined)));
    }
    return out;
  },
  // the samples of a pair that the browser storage already holds with a current index (the same members: names, sizes, dates): their entries of `list`, else null (a part of them only, an engine without natsame); the desktop app answers natsame from its own store
  // every name of the list is a candidate: a sample named by its DATA name does not start with the stem of the pair
  // `samples` (optional) is the caller's thunk for rb.nativeSamples(mem): the pair is then parsed once for the check and for the caller
  async cached(rb, mem, list, samples) {
    if (!rb.nativeSame) return null;
    const cands = list.map(f => f.name); if (!cands.length) return null;
    const hits = await rb.nativeSame(mem, cands); if (!hits.length) return null;
    const s = (await (samples ? samples() : rb.nativeSamples(mem))).samples;
    return hits.length === s.length ? list.filter(f => hits.includes(f.name)) : null;
  },
  // «from the cache» on the row of each sample of `hit` (entries of the list)
  cachedRows(hit) { for (const x of hit) { CONVROWS.add(x.name, "open"); CONVROWS.finish(x.name, "done", { scans: x.n_spectra || x.scans || 0, kind: x.kind, cached: true }); } return hit.length > 0; },
  // one sample per dataset (a .raw, a .D): when `cached` finds it, its row says «from the cache» and true is returned (nothing to load)
  async cachedOne(rb, mem, list) { return NATIVO.cachedRows(await NATIVO.cached(rb, mem, list, async () => ({ samples: [{ id: 1 }] })).catch(() => null) || []); },
  // the same for a dataset the readers module wrote as mzML in this page (it has no stub in OPFS for natsame): the sample of `list` whose source members (CONV.sources: names, sizes, dates) are these; lost with the page
  cachedModulo(mem, list) {
    const id = (/** @type {{ name: string, blob: any }[]} */ a) => a.map(m => m.name + "|" + m.blob.size + "|" + (m.blob.lastModified || 0)).sort().join("\n"), want = id(mem);
    return NATIVO.cachedRows(list.filter(x => { const s = CONV.sources.get(x.name); return !!s && s.mem.length === mem.length && id(s.mem) === want; }));
  },
  // the text key of a phase of the loading of a sample (the event mzlab-index carries `phase`), for CONVROWS and the rows of the loading
  rowState(phase) { return { read: "nread", open: "nopen", scans: "nscan", chrom: "nchrom" }[phase] || "index"; },
};

if (typeof module !== "undefined") module.exports = NATIVO;
