// Prints replay(request) of the mzdraw .wasm (wasm-bindgen --target nodejs); a refusal goes to stderr with exit code 1. See tests/test_mzdraw_wasm.py.
// usage: node mzdraw_wasm_node.js <pkg dir> <request JSON text>
const path = require("path");
const w = require(path.join(process.argv[2], "mzdraw_wasm.js"));
try {
  process.stdout.write(w.replay(process.argv[3]));
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
