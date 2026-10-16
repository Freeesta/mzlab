// Runs the table bridge of mzFinder (crates/mzlab-feat-table-wasm, wasm-bindgen --target nodejs) and prints what the native example `tabella` prints: the
// SHA-256 of every file written (see tests/test_feat_table_wasm.py). The reads come from files, as the worker's come from OPFS.
// usage: node feat_table_node.js <pkg dir> <out dir> <table.mzfeat> <campioni.json> <spec.json|-> <polarity> [file.mzidx...]
const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const [pkg, out, table, samplesPath, specPath, polarity, ...idx] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_table_wasm.js"));
const bytes = fs.readFileSync(table);
const samples = fs.readFileSync(samplesPath, "utf8");
const spec = specPath === "-" ? "" : fs.readFileSync(specPath, "utf8");
const sha = crypto.createHash("sha256").update(bytes).digest("hex");
const readTable = (_, off, len) => new Uint8Array(bytes.subarray(off, off + len));
const fds = idx.map(f => fs.openSync(f, "r"));
const sizes = Float64Array.from(idx, f => fs.statSync(f).size);
const readIdx = (k, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
const put = (name, text) => {
  fs.writeFileSync(path.join(out, name), text);
  console.log(`${name} ${crypto.createHash("sha256").update(text).digest("hex")}`);
};
const pol = Number(polarity);
put("view.json", w.table_view(readTable, bytes.length, samples, spec));
put("report.json", w.table_report(readTable, bytes.length, samples, spec, sha));
put("qc.json", w.table_qc(readTable, bytes.length, samples, spec));
for (const [name, kind] of [["full.csv", "full"], ["full_mzmine.csv", "full_mzmine"], ["quant.csv", "quant"], ["features.csv", "csv_semicolon"], ["features.rows.json", "xlsx_rows"]])
  put(name, w.export_table(readTable, bytes.length, samples, spec, sha, pol, kind));
for (const [name, kind] of [["ms2.mgf", "mgf"], ["sirius.ms", "sirius"]])
  put(name, w.export_ms2(readTable, bytes.length, samples, spec, sha, pol, kind, readIdx, sizes));
put("riga.json", w.table_row(readTable, bytes.length, samples, 3));
put("riga_tracce.json", w.table_row_traces(readTable, bytes.length, samples, 3, readIdx, sizes, 5.0));
