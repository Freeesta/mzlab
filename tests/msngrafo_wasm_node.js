// Runs msn_grafo of the msngrafo .wasm (wasm-bindgen --target nodejs) and prints the msngrafo/1; see tests/test_msngrafo_wasm.py.
// usage: node msngrafo_wasm_node.js <pkg dir> <file.mzidx> <file.mzML> <query string> [export|lib <library file>]   (export: msn_grafo_export, the spectra as text; lib: msnlib_aggiungi with the library file given, "-" = none; prints the JSON answer)
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_msngrafo_wasm.js"));
const reader = buf => (off, len) => new Uint8Array(buf.subarray(off, off + len));
const idx = fs.readFileSync(process.argv[3]);
const mzml = fs.readFileSync(process.argv[4]);
if (process.argv[6] === "lib") {
  const lib = process.argv[7] === "-" ? "" : fs.readFileSync(process.argv[7], "utf8");
  process.stdout.write(w.msnlib_aggiungi(reader(idx), idx.length, reader(mzml), path.basename(process.argv[4]), process.argv[5], lib, "STD-0001"));
} else process.stdout.write((process.argv[6] === "export" ? w.msn_grafo_export : w.msn_grafo)(reader(idx), idx.length, reader(mzml), path.basename(process.argv[4]), process.argv[5]));
