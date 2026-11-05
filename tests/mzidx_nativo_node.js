// The .mzidx the engine (mzlab-wasm, `build_mzidx_native`, what the site writes in OPFS) builds for sample C of a dataset whose members are files on disk.
// node tests/mzidx_nativo_node.js PKG_DIR OUT.mzidx C NAME MEMBER...   (MEMBER: a path; its name in the dataset is its file name)
const fs = require("fs"), path = require("path");
const [pkg, out, c, name, ...members] = process.argv.slice(2);
const w = require(path.resolve(pkg, "mzlab_wasm.js"));
const fds = members.map(m => fs.openSync(m, "r")), sizes = members.map(m => fs.statSync(m).size);
const read = (i, o, n) => { const b = Buffer.alloc(n); fs.readSync(fds[i], b, 0, n, o); return new Uint8Array(b.buffer, b.byteOffset, n); };
const fd = fs.openSync(out, "w");
const write = (o, d) => { fs.writeSync(fd, d, 0, d.length, o); };
const stats = w.build_mzidx_native(members.map(m => path.basename(m)), Float64Array.from(sizes), read, Number(c), name, false, write, 256, () => {});
fs.closeSync(fd);
process.stdout.write(stats);
