// Prints sdf(request) of the molkit .wasm (wasm-bindgen --target nodejs), with no added newline; see tests/test_molkit_sdf.py.
// usage: node molkit_sdf_node.js <pkg dir> <request JSON text>
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_mol_wasm.js"));
try {
  process.stdout.write(w.sdf(process.argv[3]));
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
