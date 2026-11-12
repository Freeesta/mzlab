// Runs the catalogue identity module (crates/mzlab-molid-wasm, wasm-bindgen --target nodejs) and prints `identity(request)` (see tests/test_molid_wasm.py).
// usage: node molid_wasm_node.js <pkg dir> <request.json>; an engine error is printed on stderr with exit code 1
const fs = require("fs"), path = require("path");
const [pkg, req] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_molid_wasm.js"));
try {
  process.stdout.write(w.identity(fs.readFileSync(req, "utf8")) + "\n");
} catch (e) {
  process.stderr.write(String(e) + "\n");
  process.exit(1);
}
