// Prints the answer of flow_curve of the flowcal .wasm (wasm-bindgen --target nodejs); see tests/test_flowcal_curve_wasm.py.
// usage: node flowcal_wasm_node.js <pkg dir> <params json> <responses.csv> <standards.csv>
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_flowcal_wasm.js"));
process.stdout.write(w.flow_curve(process.argv[3], fs.readFileSync(process.argv[4]), fs.readFileSync(process.argv[5])));
