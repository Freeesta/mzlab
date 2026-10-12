// Runs the pipeline of the mzFinder .wasm (wasm-bindgen --target nodejs, both features) on .mzidx files and prints what the native example `nativo` prints:
// the SHA-256 of every .mzfeatf and of the .mzfeat (see tests/test_feat_wasm.py). The reads come from files, as the worker's come from OPFS.
// usage: node feat_wasm_node.js <pkg dir> <out dir> <file.mzidx>... [--recipe JSON] [--budget-mb N] [--parent FORMULA --rules regole_tp.json --parent-mz M --parent-rt R]
// With --budget-mb the steps run on the sink API (outputs written at offsets, the series read by offsets, slabs and streams under that budget).
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const [pkg, out, ...rest] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_wasm.js"));

const files = [];
const opt = { recipe: "", parent: "", rules: "", "parent-mz": NaN, "parent-rt": NaN, "budget-mb": "" };
for (let i = 0; i < rest.length; i++) {
  if (rest[i].startsWith("--")) opt[rest[i].slice(2)] = rest[++i]; else files.push(rest[i]);
}
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const fds = files.map(f => fs.openSync(f, "r"));
const sizes = Float64Array.from(files, f => fs.statSync(f).size);
const read = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
const base = f => path.basename(f);

// a sink: write(offset, bytes) into a file of the out directory; the step returns {bytes, sha256}
const sink = name => { const fd = fs.openSync(path.join(out, name), "w"); return { fd, write: (off, b) => fs.writeSync(fd, b, 0, b.length, off) }; };
const budget = opt["budget-mb"] === "" ? null : Number(opt["budget-mb"]);
const stored = budget !== null ? [] : files.map((f, k) => {
  const b = w.detect_file((_, off, len) => read(k, off, len), sizes[k], base(f), opt.recipe);
  fs.writeFileSync(path.join(out, base(f) + ".mzfeatf"), b);
  console.log(`${base(f)}.mzfeatf ${sha(b)}`);
  return b;
});
let table = null;
if (budget !== null) {
  files.forEach((f, k) => {
    const s = sink(base(f) + ".mzfeatf");
    const done = JSON.parse(w.detect_file_to((_, off, len) => read(k, off, len), sizes[k], base(f), opt.recipe, s.write));
    fs.closeSync(s.fd);
    console.log(`${base(f)}.mzfeatf ${done.sha256}`);
    stored.push(new Uint8Array(fs.readFileSync(path.join(out, base(f) + ".mzfeatf"))));
  });
  const s = sink("series.mzfeat");
  const done = JSON.parse(w.align_series_to(stored, read, sizes, opt.recipe, s.write, budget));
  fs.closeSync(s.fd);
  console.log(`series.mzfeat ${done.sha256}`);
} else {
  table = w.align_series(stored, read, sizes, opt.recipe);
  fs.writeFileSync(path.join(out, "series.mzfeat"), table);
  console.log(`series.mzfeat ${sha(table)}`);
}
if (opt.parent) {
  const prt = Number(opt["parent-rt"]);
  const h = w.run_hypotheses(stored, read, sizes, fs.readFileSync(opt.rules), opt.parent, prt, opt.recipe);
  console.log(`hypotheses ${h.tried()} tried, ${h.found()} found`);
  const minutes = Float64Array.from(files, f => { const m = /_t(\d+)min/.exec(base(f)); return m ? Number(m[1]) : NaN; });
  if (budget !== null) {
    const fd = fs.openSync(path.join(out, "series.mzfeat"), "r");
    const tsize = fs.fstatSync(fd).size;
    const s = sink("series_evid.mzfeat");
    const done = JSON.parse(w.build_evidence_to((_, off, len) => { const b = new Uint8Array(len); fs.readSync(fd, b, 0, len, off); return b; },
      tsize, w.load_hypotheses(h.state()), minutes, Number(opt["parent-mz"]), prt, opt.recipe, s.write, budget));
    fs.closeSync(s.fd);
    console.log(`series_evid.mzfeat ${done.sha256}`);
  } else {
    const ev = w.build_evidence(table, h, minutes, Number(opt["parent-mz"]), prt, opt.recipe);
    fs.writeFileSync(path.join(out, "series_evid.mzfeat"), ev);
    console.log(`series_evid.mzfeat ${sha(ev)}`);
  }
}
