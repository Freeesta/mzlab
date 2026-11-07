// Prints expectations(smiles) of the msnregole .wasm (wasm-bindgen --target nodejs); see tests/test_msnregole_wasm.py.
// usage: node msnregole_wasm_node.js <pkg dir> <smiles>
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_msnregole_wasm.js"));
process.stdout.write(w.expectations(process.argv[3]));
