// Opens N copies of one synthetic .mzidx in ONE engine, each in its own file slot, under the memory the browser grants (`node --wasm-max-mem-pages=P`).
// Prints one JSON line: per file {ok|error key|trap}, the linear memory after each open and the cache bytes (docs/agenti/memory-limits.md).
// usage: node [--wasm-max-mem-pages=P] memoria_limite_node.js <pkg engine> <budget MB> <copies> <file.mzidx>
const fs = require("fs");
const path = require("path");
const [engPkg, budget, copies, file] = process.argv.slice(2);
const eng = require(path.join(engPkg, "mzlab_wasm.js"));
const fd = fs.openSync(file, "r");
const size = fs.statSync(file).size;
const read = (off, len) => { const b = new Uint8Array(len); fs.readSync(fd, b, 0, len, off); return b; };
const e = new eng.Engine();
e.set_budget_mb(Number(budget));
const files = [];
let peak = 0;
for (let k = 0; k < Number(copies); k++) {
  try {
    const r = JSON.parse(e.open_index(k, size, read));
    files.push({ k, ok: true, scans: r.scans });
  } catch (x) {
    // the engine throws a JSON error object (a key), or a WebAssembly trap (RuntimeError)
    const t = typeof x === "string" ? x : (x && x.message) || String(x);
    let key = null;
    try { key = JSON.parse(t); } catch (_) {}
    if (key === null && process.env.TRAP_STACK) console.error(x && x.stack);
    files.push({ k, ok: false, trap: key === null, error: key ? key.error_key : t.slice(0, 80), params: key ? key.params : null });
    if (key === null) break; // a trap leaves the instance unusable
  }
  peak = Math.max(peak, eng.memory_bytes ? eng.memory_bytes() : 0);
}
console.log(JSON.stringify({ budget_mb: Number(budget), copies: Number(copies), mem_mb: Math.round(peak / 1048576), files }));
