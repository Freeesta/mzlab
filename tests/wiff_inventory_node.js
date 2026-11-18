// Runs `wiff_list` or `wiff_verify` of the `wiffwrite` module (wasm-bindgen --target nodejs) on .wiff + .wiff.scan pairs and prints the text:
//   node wiff_inventory_node.js PKG list [--inventory] FILE.wiff...    |    node wiff_inventory_node.js PKG verify OUT.wiff SRC.wiff...
// Used by tests/test_wiff_inventory_wasm.py (the same bytes as `mzlab-cli wiff list|verify`).
const fs = require("fs");
const path = require("path");
const m = require(path.resolve(process.argv[2]));
const verb = process.argv[3];
let rest = process.argv.slice(4);
const full = rest[0] === "--inventory";
if (full) rest = rest.slice(1);
const members = [];
for (const w of rest) for (const f of [w, w + ".scan"]) members.push({ name: path.basename(f), data: fs.readFileSync(f) });
const read = (i, off, n) => new Uint8Array(members[i].data.subarray(off, off + n));
const names = members.map((x) => x.name);
const sizes = Float64Array.from(members.map((x) => x.data.length));
try {
  process.stdout.write(verb === "list" ? m.wiff_list(names, sizes, full, read) : m.wiff_verify(names, sizes, read));
} catch (e) {
  process.stderr.write(String(e));
  process.exit(2);
}
