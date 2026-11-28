// Prints the answer of flow_kinetics of the flowkin .wasm (wasm-bindgen --target nodejs); see tests/test_flowkin_wasm.py.
// usage: node flowkin_node.js <pkg dir> <params json> <areas.csv> <samples.json> [<uv.csv>]
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_flowkin_wasm.js"));
const uv = process.argv[6] ? fs.readFileSync(process.argv[6]) : new Uint8Array(0);
process.stdout.write(w.flow_kinetics(process.argv[3], fs.readFileSync(process.argv[4]), fs.readFileSync(process.argv[5]), uv));
