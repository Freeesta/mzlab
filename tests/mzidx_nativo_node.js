// The .mzidx the engine (mzlab-wasm, `build_mzidx_native`, what the site writes in OPFS) builds for sample C of a dataset whose members are files on disk.
// node tests/mzidx_nativo_node.js PKG_DIR OUT.mzidx C NAME MEMBER...   (MEMBER: a path; its name in the dataset is its file name)
// MZIDX_PRINTS: "given" (the page took them, `Passata`), "late" (a function the build calls after its first pass), "bad" (a late function giving nothing);
// unset: the engine takes them.
const fs = require("fs"), path = require("path");
const [pkg, out, c, name, ...members] = process.argv.slice(2);
const w = require(path.resolve(pkg, "mzlab_wasm.js"));
const fds = members.map(m => fs.openSync(m, "r")), sizes = members.map(m => fs.statSync(m).size);
const read = (i, o, n) => { const b = Buffer.alloc(n); fs.readSync(fds[i], b, 0, n, o); return new Uint8Array(b.buffer, b.byteOffset, n); };
const fd = fs.openSync(out, "w");
// MZIDX_VIEWS: "count" (each write must be a view of the engine's memory: its buffer is larger than its bytes), "keep" (the arrays kept, written
// only at the end: what a callback that keeps them without copying would get)
const viewsMode = process.env.MZIDX_VIEWS, kept = [];
let views = 0, writes = 0;
const write = (o, d) => { writes++; if (d.buffer.byteLength > d.length) views++; if (viewsMode === "keep") kept.push([o, d]); else fs.writeSync(fd, d, 0, d.length, o); };
const taken = () => members.flatMap(m => { const p = new w.Passata(fs.statSync(m).size), b = fs.readFileSync(m);
  for (let o = 0; o < b.length; o += 8 << 20) p.aggiorna(b.subarray(o, o + (8 << 20)));
  return Array.from(p.fine()); });
const mode = process.env.MZIDX_PRINTS, called = () => process.stderr.write("late prints asked\n"), late = { late: () => (called(), taken()), bad: () => (called(), null) };
const prints = mode === "given" ? taken() : late[mode] || null;
const stats = w.build_mzidx_native(members.map(m => path.basename(m)), Float64Array.from(sizes), read, Number(c), name, false, write, 256, () => {}, prints);
for (const [o, d] of kept) fs.writeSync(fd, d, 0, d.length, o);
if (viewsMode) process.stderr.write(`views ${views} of ${writes}\n`);
fs.closeSync(fd);
process.stdout.write(stats);
