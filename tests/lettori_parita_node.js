// Runs `to_mzml` of one readers module (wasm-bindgen --target nodejs) over a dataset on the disk and writes the mzML it sends to its sink; see tests/test_lettori_parita.py.
// usage: node lettori_parita_node.js <pkg dir> <plan.json {members:[{name,path}], out}>
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_lettori_wasm.js"));
const plan = JSON.parse(fs.readFileSync(process.argv[3], "utf8"));
const fds = plan.members.map(m => fs.openSync(m.path, "r"));
const sizes = Float64Array.from(plan.members.map(m => fs.fstatSync(fds[plan.members.indexOf(m)]).size));
// the page's `read(member, offset, length) -> Uint8Array`, over the files
const read = (i, off, n) => {
  const b = Buffer.alloc(n);
  fs.readSync(fds[i], b, 0, n, off);
  return new Uint8Array(b);
};
const parts = [];
try {
  w.to_mzml(plan.members.map(m => m.name), sizes, read, 0, p => parts.push(Buffer.from(p)), () => {});
} catch (e) {
  console.error(String(e));
  process.exit(1);
}
fs.writeFileSync(plan.out, Buffer.concat(parts));
