// Runs search_with_fdr of the libraries .wasm (wasm-bindgen --target nodejs) and prints its JSON; see tests/test_librerie_fdr_wasm.py.
// usage: node fdr_wasm_node.js <pkg dir> <dir with lib.msp and queries.json>
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_lib_wasm.js"));
const dir = process.argv[3];

function mem() {
  let buf = Buffer.alloc(0);
  return {
    read: (off, len) => new Uint8Array(buf.subarray(off, off + len)),
    write: (off, bytes) => {
      if (off + bytes.length > buf.length) buf = Buffer.concat([buf, Buffer.alloc(off + bytes.length - buf.length)]);
      buf.set(bytes, off);
    },
    get size() { return buf.length; },
  };
}
const msp = fs.readFileSync(path.join(dir, "lib.msp"));
const scratch = mem();
const out = mem();
const imp = new w.Importer("lib.msp", msp.subarray(0, 4096), scratch.read, scratch.write);
for (let i = 0; i < msp.length; i += 65536) imp.push(msp.subarray(i, Math.min(i + 65536, msp.length)));
imp.finish(out.write);
const lib = w.Lib.open(out.read, out.size, 64e6);
const q = JSON.parse(fs.readFileSync(path.join(dir, "queries.json"), "utf8"));
process.stdout.write(lib.search_with_fdr(
  Float64Array.from(q.precs), Int32Array.from(q.pols), Uint32Array.from(q.starts),
  Float64Array.from(q.mz), Float64Array.from(q.it), 10, true, 0.01, 0));
