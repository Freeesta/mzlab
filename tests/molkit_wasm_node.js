// Prints check(request) of the molkit .wasm (wasm-bindgen --target nodejs); see tests/test_molkit_wasm.py.
// usage: node molkit_wasm_node.js <pkg dir> <request JSON text>
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
try {
  process.stdout.write(w.check(process.argv[3]) + "\n");
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
