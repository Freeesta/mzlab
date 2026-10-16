"use strict";
// Files of a maker (Thermo .raw, SCIEX .wiff + .wiff.scan, Agilent .D folder) are loaded as they are (rust-nativo.js): no mzML is made. A sample is listed as `x.mzML` (a File with the members attached:
// `mzlabNat`, read by lr.js) and goes through upload() like any mzML; «Download .mzML» writes its mzML on request. Classic script, loaded after conversione.js; the pure helpers are run by node in tests/test_nativo.py.

const NATIVO = {
  // one key for the dataset (its samples share the members in the browser storage): the name without odd characters + a random part
  key(ds) { return String(ds).replace(/[^\w.-]+/g, "_").slice(0, 60) + "-" + Math.random().toString(36).slice(2, 8); },
  // the File that stands for sample `c` (number `id`) of a dataset in the list of the files; `members` = [{ name, blob }], names relative to the dataset
  sample(name, key, members, c, id, centroid) {
    const f = new File([JSON.stringify({ mzlab: "nat1", ds: key })], name), size = members.reduce((a, m) => a + m.blob.size, 0);
    Object.defineProperty(f, "mzlabNat", { value: { key, members, c, id, centroid: !!centroid, size } });
    return f;
  },
  // the size to show: the one of the members
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
  // the text key of a phase of the loading of a sample (the event mzlab-index carries `phase`), for CONVROWS and the rows of the loading
  rowState(phase) { return { read: "nread", open: "nopen", scans: "nscan", chrom: "nchrom" }[phase] || "index"; },
};

if (typeof module !== "undefined") module.exports = NATIVO;
