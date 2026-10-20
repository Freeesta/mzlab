// Converts every chapter of mzlab/web/teoria/src through the mzlab-libro .wasm (wasm-bindgen --target nodejs) and writes the pages into <out dir>
// (see tests/test_libro_wasm.py). usage: node libro_wasm_node.js <pkg dir> <src dir> <out dir> <importati.txt>
const fs = require("fs");
const path = require("path");
const [pkg, src, out, listed] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_libro_wasm.js"));
const imported = new Set(fs.readFileSync(listed, "utf8").split("\n").map(l => l.split("#")[0].trim()).filter(Boolean));
console.log(w.version());
for (const f of fs.readdirSync(src).filter(n => n.endsWith(".md")).sort()) {
  fs.writeFileSync(path.join(out, f.replace(/\.md$/, ".html")), w.convert_chapter(fs.readFileSync(src + "/" + f, "utf8"), f, imported.has(f)));
}
try { w.convert_chapter("no heading here\n", "x.md", false); } catch (e) { console.log("error " + e); }
