// The metodo/1 of one sample of a .wiff alone, as the page asks for it: `engine` = Engine.wiff_metodo of the engine .wasm (read(offset, length)), `module` = wiff_metodo of
// the `sciex` readers module (read(member, offset, length)); see tests/test_wiff_metodo_sito.py. usage: node wiff_metodo_node.js <pkg dir> <engine|module> <file.wiff> <sample>
const fs = require("fs");
const path = require("path");
const [dir, kind, file, sample] = process.argv.slice(2);
const fd = fs.openSync(file, "r"), size = fs.fstatSync(fd).size;
const read = (off, n) => { const b = Buffer.alloc(n); fs.readSync(fd, b, 0, n, off); return new Uint8Array(b); };
const name = path.basename(file);
try {
  if (kind === "engine") {
    const w = require(path.join(dir, "mzlab_wasm.js"));
    process.stdout.write(new w.Engine().wiff_metodo(name, size, read, Number(sample), ""));
  } else {
    const w = require(path.join(dir, "mzlab_lettori_wasm.js"));
    process.stdout.write(w.wiff_metodo(name, size, (_i, off, n) => read(off, n), Number(sample), ""));
  }
} catch (e) { console.error(String(e)); process.exit(1); }
