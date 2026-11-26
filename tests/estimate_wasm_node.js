// Prints the `estimates/1` text that the mzFinder .wasm (wasm-bindgen --target nodejs) gives for one .mzidx: `flow_estimate`, the export the browser runner of `features.estimate` calls.
// usage: node estimate_wasm_node.js <pkg dir> <file.mzidx> <name of the source> [params JSON] (see tests/test_estimate_nominal_wasm.py)
const fs = require("fs");
const path = require("path");
const [pkg, file, name, params] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_wasm.js"));
const fd = fs.openSync(file, "r");
const read = (_, off, len) => { const b = new Uint8Array(len); fs.readSync(fd, b, 0, len, off); return b; };
process.stdout.write(w.flow_estimate(read, fs.statSync(file).size, name, params || "{}"));
