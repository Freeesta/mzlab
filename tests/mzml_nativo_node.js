// The mzML the engine (mzlab-wasm, `native_to_mzml`, «Download .mzML» of a sample read by the engine) writes for sample C of a dataset whose members are files on disk.
// node tests/mzml_nativo_node.js PKG_DIR OUT.mzML C MEMBER...   (MEMBER: a path; its name in the dataset is its file name)
const fs = require("fs"), path = require("path");
const [pkg, out, c, ...members] = process.argv.slice(2);
const w = require(path.resolve(pkg, "mzlab_wasm.js"));
const fds = members.map(m => fs.openSync(m, "r")), sizes = members.map(m => fs.statSync(m).size);
const read = (i, o, n) => { const b = Buffer.alloc(n); fs.readSync(fds[i], b, 0, n, o); return new Uint8Array(b.buffer, b.byteOffset, n); };
const fd = fs.openSync(out, "w");
w.native_to_mzml(members.map(m => path.basename(m)), Float64Array.from(sizes), read, Number(c), false, "standard", d => fs.writeSync(fd, d), () => {});
fs.closeSync(fd);
