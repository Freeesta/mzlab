// Times the hot paths of the `hr` .wasm (wasm-bindgen --target nodejs) on .mzidx files: detect (ion and family), preview, align, proofs (hypotheses + evidence).
// usage: node bench_hr.js <pkg dir> <reps> <file.mzidx>...   prints one JSON line {paths: {name: [ms...]}, sha: {name: sha256 of the output}} (see tools/bench_hr.py).
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const [pkg, reps, ...files] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_wasm.js"));
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const fds = files.map(f => fs.openSync(f, "r"));
const sizes = Float64Array.from(files, f => fs.statSync(f).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
const rd = k => (_, off, len) => read(k, off, len);
const rules = fs.readFileSync(path.join(__dirname, "..", "TP_Mine/data/regole_tp_2.json"));
const paths = {
  detect_ion: r => files.map((f, k) => w.detect_file(rd(k), sizes[k], path.basename(f), r || '{"mode":"ion"}')),
  detect_family: () => files.map((f, k) => w.detect_file(rd(k), sizes[k], path.basename(f), '{"mode":"family"}')),
  preview: () => [Buffer.from(w.preview_file(rd(0), sizes[0], '{"mode":"ion"}'))],
  align: () => [w.align_series(paths.stored, read, sizes, '{"mode":"ion"}')],
  proofs: () => {
    const h = w.run_hypotheses(paths.stored, read, sizes, rules, "", "C21H32N2O2", 1.0, '{"mode":"ion"}');
    return [w.build_evidence(paths.table, h, Float64Array.from(files, () => NaN), 345.253655, 1.0, '{"mode":"ion"}')];
  },
};
const out = { paths: {}, sha: {} };
paths.stored = paths.detect_ion();
paths.table = w.align_series(paths.stored, read, sizes, '{"mode":"ion"}');
for (const name of ["detect_ion", "detect_family", "preview", "align", "proofs"]) {
  paths[name]();                                   // one warm-up run, not timed
  out.paths[name] = [];
  for (let i = 0; i < Number(reps); i++) {
    const t = process.hrtime.bigint();
    const res = paths[name]();
    out.paths[name].push(Number(process.hrtime.bigint() - t) / 1e6);
    out.sha[name] = sha(Buffer.concat(res.map(x => Buffer.from(x))));
  }
}
console.log(JSON.stringify(out));
