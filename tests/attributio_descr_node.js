// Prints the JSON of `rt_descriptors(smiles)` of the attributio bridge (wasm-bindgen --target nodejs); see tests/test_attributio_descr_wasm.py.
// usage: node attributio_descr_node.js <pkg dir> <smiles>...
const path = require("path");
const [pkg, ...list] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_attributio_wasm.js"));
console.log(JSON.stringify(list.map((s) => JSON.parse(w.rt_descriptors(s)))));
