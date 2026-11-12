// Prints check(request) of the molkit .wasm (wasm-bindgen --target nodejs), or substructure(request) / cip(request) when the third argument
// is "sub" / "cip"; see tests/test_molkit_wasm.py. usage: node molkit_wasm_node.js <pkg dir> <request JSON text> [sub|cip]
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
const fn = { cip: w.cip, sub: w.substructure }[process.argv[4]] || w.check;
try {
  process.stdout.write(fn(process.argv[3]) + "\n");
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
