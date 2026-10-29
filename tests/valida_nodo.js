// Runs `valida_mzml` of the `valida` module (wasm-bindgen --target nodejs) on the files given and prints the text of each: node valida_nodo.js PKG FILE...
// Used by tests/test_valida_wasm.py (the same bytes as `mzlab-cli validate --json`).
const fs = require("fs");
const path = require("path");
const m = require(path.resolve(process.argv[2]));
for (const f of process.argv.slice(3)) {
  const data = fs.readFileSync(f);
  const read = (i, off, n) => new Uint8Array(data.subarray(off, off + n));
  process.stdout.write(m.valida_mzml(path.basename(f), data.length, read));
}
