// Prints check(request) of the molkit .wasm (wasm-bindgen --target nodejs), or substructure(request) when the third argument is "sub"; see
// tests/test_molkit_wasm.py. usage: node molkit_wasm_node.js <pkg dir> <request JSON text> [sub]
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
try {
  process.stdout.write((process.argv[4] === "sub" ? w.substructure : w.check)(process.argv[3]) + "\n");
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
