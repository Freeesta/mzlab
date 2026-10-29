// node node_bench.mjs VARIANT_DIR REPS DATA_DIR: one JSON line {variant, compile_ms, inst_ms, ms:{job}, check:{job}}
import fs from "fs"; import { createRequire } from "module"; import { pathToFileURL } from "url"; import path from "path";
const require = createRequire(import.meta.url);
const { suite } = require("./suite.js");
const dir = process.argv[2], reps = +process.argv[3] || 5, data = process.argv[4];
const bytes = fs.readFileSync(dir + "/wbench_bg.wasm");
let a = performance.now(); const mod = await WebAssembly.compile(bytes); const tc = performance.now() - a;
const m = await import(pathToFileURL(path.resolve(dir, "wbench.js")).href);
a = performance.now(); m.initSync({ module: mod }); const ti = performance.now() - a;
const files = { hr: fs.readFileSync(data + "/hr_prof.mzML"), lr: fs.readFileSync(data + "/B_FullMass-t15.mzML") };
if (fs.existsSync(data + "/x.raw")) files.raw = fs.readFileSync(data + "/x.raw");
const r = await suite(m, files, reps, () => {});
console.log(JSON.stringify({ variant: path.basename(dir), compile_ms: +tc.toFixed(1), inst_ms: +ti.toFixed(1),
  ms: Object.fromEntries(Object.entries(r).map(([k, v]) => [k, v.ms])), check: Object.fromEntries(Object.entries(r).map(([k, v]) => [k, v.v])) }));
