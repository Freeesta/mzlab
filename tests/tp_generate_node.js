// Prints the hypotheses/3 text that tp_generate of the mzlab-feat-wasm .wasm (wasm-bindgen --target nodejs) gives for a formula parent: the call of the browser runner of `tp.generate`.
// usage: node tp_generate_node.js <pkg dir> <rules json file> <parent formula> <params json> (see tests/test_tp_generate_wasm.py)
const fs = require("fs");
const path = require("path");
const [pkg, rules, parent, params] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_wasm.js"));
process.stdout.write(w.tp_generate(fs.readFileSync(rules), parent, "", params));
