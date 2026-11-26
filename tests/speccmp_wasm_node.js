// Runs `spectrum_compare` of the library .wasm (mzlab-lib-wasm, wasm-bindgen --target nodejs) on one request and prints {s: text} or {e: error text}; see tests/test_spectrum_compare_wasm.py.
// usage: node speccmp_wasm_node.js <pkg dir> <params.json file> <sides.json file>
const fs = require("fs");
const path = require("path");
const { spectrum_compare } = require(path.join(process.argv[2], "mzlab_lib_wasm.js"));
let out;
try { out = { s: spectrum_compare(fs.readFileSync(process.argv[3], "utf8"), fs.readFileSync(process.argv[4], "utf8")) }; } catch (x) { out = { e: String(x) }; }
process.stdout.write(JSON.stringify(out));
