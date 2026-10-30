// Runs a plan of routes on the engine .wasm (mzlab-wasm, wasm-bindgen --target nodejs) and prints one answer per step as JSON; see tests/test_parita_rotte_app.py.
// usage: node rotte_wasm_node.js <pkg dir> <file.mzidx> <file.mzML> <plan.json>
// a step: {op, slot, ...}; an answer: {s: text} | {f64: [numbers]} | {b: hex} | {e: error text}
const fs = require("fs");
const path = require("path");
const wasm = require(path.join(process.argv[2], "mzlab_wasm.js"));
const { Engine } = wasm;
const idx = fs.readFileSync(process.argv[3]), mzml = fs.readFileSync(process.argv[4]);
const plan = JSON.parse(fs.readFileSync(process.argv[5], "utf8"));
const reader = buf => (off, len) => new Uint8Array(buf.subarray(off, off + len));
const rdIdx = reader(idx), rdMzml = reader(mzml);
const e = new Engine();
// as rust-worker.js: count and spectra_len answer 0xFFFFFFFF for a slot with no run open, and the worker turns it into err.file.raw
const NO_RUN = 0xFFFFFFFF, noRun = () => JSON.stringify({ error_key: "err.file.raw", params: {} });
const runCount = s => { const n = e.count(s.slot, s.level); if (n === NO_RUN) throw noRun(); return n; };
const trace = (n, out) => ({ f64: Array.from(out.subarray(0, n)) });
const run = s => {
  switch (s.op) {
    case "idxopen": return { s: e.open_index(s.slot, idx.length, rdIdx) };
    case "route": return { s: e.route(s.slot, s.path, s.query || "") };
    case "fileinfo": return { s: e.fileinfo(s.slot, s.name, mzml.length, rdMzml) };
    case "canali": return { s: e.canali(s.slot, s.name, mzml.length, rdMzml) };
    case "mrm": return { s: e.mrm(s.slot, mzml.length, rdMzml) };
    case "mrmchrom": return { s: e.mrm_chrom(s.slot, mzml.length, rdMzml, s.kind) };
    case "mzmlroute": return { s: e.mzml_route(s.slot, s.path, s.query || "", rdMzml) };
    case "scanbin": return { b: Buffer.from(e.scanbin(s.slot, s.query || "", rdMzml)).toString("hex") };
    case "specexact": return { s: e.spectrum_exact(s.slot, s.query || "", rdMzml) };
    case "map": return { s: e.map_route(s.path, s.items, s.query || "") };
    case "diff": return { s: e.diff_traces(s.path, s.query || "") };
    case "flow_command": return { s: wasm.flow_command(s.body) };
    case "lr_stat": return { s: wasm.stat_post(s.path, s.body) };
    case "lr_campioni": return { s: wasm.campioni(s.cop, s.body) };
    case "open": return { s: e.open(s.slot, mzml.length, rdMzml) };
    case "tic": { const out = new Float64Array(2 * runCount(s)); return trace(2 * e.tic(s.slot, s.level, s.px || 0, out), out); }
    case "xic": { const out = new Float64Array(2 * runCount(s)); return trace(2 * e.xic(s.slot, s.level, s.mz, s.tol, s.px || 0, out), out); }
    case "spectra": { const n = e.spectra_len(s.slot, s.level, s.i0, s.i1); if (n === NO_RUN) throw noRun(); const out = new Float64Array(n); return trace(e.spectra(s.slot, s.level, s.i0, s.i1, s.bin, out), out); }
    default: throw new Error("unknown step " + s.op);
  }
};
const answers = plan.map(s => { try { return run(s); } catch (x) { return { e: typeof x === "string" ? x : String(x && x.message || x) }; } });
process.stdout.write(JSON.stringify(answers));
