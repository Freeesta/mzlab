// Prints check(request), or cip(request) with a third argument "cip", of the molkit .wasm (wasm-bindgen --target nodejs); see tests/test_molkit_wasm.py.
// usage: node molkit_wasm_node.js <pkg dir> <request JSON text> [check|cip]
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
const fn = process.argv[4] === "cip" ? w.cip : w.check;
try {
  process.stdout.write(fn(process.argv[3]) + "\n");
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
