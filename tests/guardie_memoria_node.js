// Peak of the linear memory of the .wasm after each phase, on the synthetic series of tools/feat_mirato.py (profile `piccolo`) and its .mzidx: the growth of the
// WASM memory is deterministic for a given input, so it is a guard of the architecture, not a measure of speed (docs/agenti/memoria.md). Prints JSON { phase: bytes }.
// usage: node guardie_memoria_node.js <pkg feat> <pkg engine> <out dir> <file.mzidx>...
const fs = require("fs");
const path = require("path");
const [featPkg, engPkg, out, ...files] = process.argv.slice(2);
const feat = require(path.join(featPkg, "mzlab_feat_wasm.js"));
const eng = require(path.join(engPkg, "mzlab_wasm.js"));
const fds = files.map(f => fs.openSync(f, "r"));
const sizes = Float64Array.from(files, f => fs.statSync(f).size);
const readK = k => (_, off, len) => { const b = new Uint8Array(len); fs.readSync(fds[k], b, 0, len, off); return b; };
const read = (k, off, len) => readK(k)(0, off, len);
const peaks = {};
const mark = (name, mem) => { peaks[name] = Math.max(peaks[name] || 0, mem()); };
const sink = name => { const fd = fs.openSync(path.join(out, name), "w"); return { fd, write: (off, b) => fs.writeSync(fd, b, 0, b.length, off) }; };
const featMem = () => feat.memory_bytes(), engMem = () => eng.memory_bytes();
const base = f => path.basename(f);
const recipe = '{"mode":"ion"}';
const budget = Number(process.env.BUDGET_MB || 192); // a small one makes align and proofs take the way in slabs

const stored = files.map((f, k) => {
  const s = sink(base(f) + ".mzfeatf");
  feat.detect_file_to(readK(k), sizes[k], base(f), recipe, s.write); fs.closeSync(s.fd); mark("detect", featMem);
  return new Uint8Array(fs.readFileSync(path.join(out, base(f) + ".mzfeatf")));
});
let s = sink("series.mzfeat");
feat.align_series_to(stored, read, sizes, recipe, s.write, budget); fs.closeSync(s.fd); mark("align", featMem);
const h = feat.run_hypotheses(stored, read, sizes, fs.readFileSync(process.env.RULES), "", "C21H32N2O2", 1.0, recipe); mark("hypotheses", featMem);
const fd = fs.openSync(path.join(out, "series.mzfeat"), "r");
s = sink("series_evid.mzfeat");
const minutes = Float64Array.from(files, f => { const m = /_t(\d+)min/.exec(base(f)); return m ? Number(m[1]) : NaN; });
feat.build_evidence_to((_, off, len) => { const b = new Uint8Array(len); fs.readSync(fd, b, 0, len, off); return b; }, fs.fstatSync(fd).size, h, minutes, 345.253655, 1.0, recipe, s.write, budget);
fs.closeSync(s.fd); mark("proofs", featMem);

// the engine: the index of a file open, its TIC and an XIC, a scan list, then the whole map and a region of the session (the cache is one LRU)
const e = new eng.Engine();
e.open_index(0, sizes[0], (off, len) => readK(0)(0, off, len)); mark("engine_open", engMem);
e.route(0, "/api/chrom", "k=0&kind=tic&level=1"); e.index_xic(0, 1, 345.2536, 0.35); mark("engine_tic_xic", engMem);
e.route(0, "/api/scanlist", "k=0&level=1");  mark("engine_scanlist", engMem);
const items = JSON.stringify([{ slot: 0, label: "a", time: 0, full: true }]);
e.map_route("/api/map", items, "k=0&level=1"); e.map_route("/api/map", items, "k=0&level=1&rt0=1&rt1=20&mz0=200&mz1=400&nrt=1024&nmz=512"); e.trim_cache(); mark("engine_map", engMem);
// subxic of the survey, and the same answers of a second engine with a tiny budget (every cache cut to nothing after each answer): the bytes must not change,
// and a tiny budget never ends in a trap (the answers below would throw)
const asks = x => [x.route(0, "/api/chrom", "k=0&kind=tic&level=1"), x.route(0, "/api/scanlist", "k=0&level=1"), x.index_xic(0, 1, 345.2536, 0.35),
  x.map_route("/api/map", items, "k=0&level=1"), x.map_route("/api/map", items, "k=0&level=1&rt0=1&rt1=20&mz0=200&mz1=400&nrt=1024&nmz=512"),
  x.subxic(0, 0, "f=C21H32N2O2&ppm=5&z=1")];
const whole = asks(e); mark("engine_subxic", engMem);
const small = new eng.Engine(); small.set_budget_mb(0.001);
small.open_index(0, sizes[0], (off, len) => readK(0)(0, off, len));
const tiny = asks(small); small.trim_cache(); mark("engine_tiny_budget", engMem);
whole.forEach((a, i) => {
  if (!a) throw new Error("the engine does not answer " + i); if (a !== tiny[i]) throw new Error("the answer " + i + " changes with a tiny budget"); });
if (small.cache_bytes() !== 0) throw new Error("a tiny budget leaves " + small.cache_bytes() + " bytes in the cache");
console.log(JSON.stringify(peaks));
