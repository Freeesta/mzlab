// Prints substructure(request) of the sub .wasm (wasm-bindgen --target nodejs); see tests/test_sub_wasm.py.
// usage: node sub_wasm_node.js <pkg dir> <request file>
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_sub_wasm.js"));
try {
  process.stdout.write(w.substructure(fs.readFileSync(process.argv[3], "utf8")));
} catch (e) {
  process.stderr.write(String(e));
  process.exit(1);
}
