// «Download .mzML» of a .wiff sample as the engine worker writes it (rust-nativo.js toMzml): `to_mzml_come` of the `sciex` module with no optical matrix.
// node tests/mzml_scarica_node.js PKG_DIR OUT.mzML C COMPRESSION MEMBER...   (MEMBER: a path; its name in the dataset is its file name)
const fs = require("fs"), path = require("path");
const [pkg, out, c, comp, ...members] = process.argv.slice(2);
const w = require(path.resolve(pkg, "mzlab_lettori_wasm.js"));
const fds = members.map(m => fs.openSync(m, "r")), sizes = members.map(m => fs.statSync(m).size);
const read = (i, o, n) => { const b = Buffer.alloc(n); fs.readSync(fds[i], b, 0, n, o); return new Uint8Array(b.buffer, b.byteOffset, n); };
const fd = fs.openSync(out, "w");
w.to_mzml_come(members.map(m => path.basename(m)), Float64Array.from(sizes), read, Number(c), false, comp, p => { fs.writeSync(fd, p); }, () => {});
fs.closeSync(fd);
